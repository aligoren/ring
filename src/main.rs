use std::env;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::io;
use rand::Rng;
use socket2::{Domain, Protocol, Socket, Type};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(unix)]
use libc::SOCK_RAW;

#[cfg(windows)]
const SOCK_RAW: i32 = 3;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run <target> [options]");
        println!("Example: cargo run 8.8.8.8 -c 5 -s 64 -w 1000 -ttl 128 -4");
        return;
    }

    let target = &args[1];
    let count = get_argument(&args, "-c", 4);
    let packet_size = get_argument(&args, "-s", 56) as usize;
    let timeout = get_argument(&args, "-w", 1000);
    let ttl = get_argument(&args, "-ttl", 128);
    let continuous = args.contains(&"-t".to_string());

    println!("ringing {} with {} bytes of data:", target, packet_size);

    let target_ip = match target.parse::<IpAddr>() {
        Ok(ip) => ip,
        Err(_) => match resolve_target(target) {
            Ok(ip) => ip,
            Err(e) => {
                println!("Invalid target address: {}", e);
                return;
            }
        },
    };

    run_ring(target_ip, count, packet_size, timeout, ttl, continuous);
}

fn get_argument(args: &[String], option: &str, default: i32) -> i32 {
    if let Some(index) = args.iter().position(|arg| arg == option) {
        if let Some(value) = args.get(index + 1) {
            if let Ok(num) = value.parse::<i32>() {
                return num;
            }
        }
    }
    default
}

fn run_ring(target: IpAddr, mut count: i32, packet_size: usize, timeout: i32, ttl: i32, continuous: bool) {
    let packet = create_icmp_packet(packet_size, target);
    let socket = create_socket(target, ttl, timeout).expect("Failed to create socket");

    let dest_addr = match target {
        IpAddr::V4(ip) => SocketAddr::new(IpAddr::V4(ip), 0),
        IpAddr::V6(ip) => SocketAddr::new(IpAddr::V6(ip), 0),
    };

    let mut sent = 0;
    let mut received = 0;
    let mut min_rtt = Duration::MAX;
    let mut max_rtt = Duration::ZERO;
    let mut total_rtt = Duration::ZERO;

    while continuous || count > 0 {
        let result = send_and_receive_ring(&socket, &packet, &dest_addr, timeout);

        if let Ok(rtt) = result {
            received += 1;
            total_rtt += rtt;
            min_rtt = min_rtt.min(rtt);
            max_rtt = max_rtt.max(rtt);

            println!(
                "Reply from {}: bytes={} time={}ms TTL={}",
                target,
                packet_size,
                rtt.as_millis(),
                ttl
            );
        } else {
            println!("Request timed out.");
        }

        sent += 1;
        if !continuous {
            count -= 1;
        }

        if count > 0 || continuous {
            sleep(Duration::from_secs(1));
        }
    }

    println!("\nring statistics for {}:", target);
    println!(
        "    Packets: Sent = {}, Received = {}, Lost = {} ({:.0}% loss),",
        sent,
        received,
        sent - received,
        if sent > 0 {
            100.0 * (sent - received) as f32 / sent as f32
        } else {
            0.0
        }
    );

    if received > 0 {
        println!("Approximate round trip times in milli-seconds:");
        println!(
            "    Minimum = {}ms, Maximum = {}ms, Average = {}ms",
            min_rtt.as_millis(),
            max_rtt.as_millis(),
            total_rtt.as_millis() / received as u128
        );
    }
}

fn create_socket(target: IpAddr, ttl: i32, timeout: i32) -> io::Result<Socket> {
    let domain = match target {
        IpAddr::V4(_) => Domain::IPV4,
        IpAddr::V6(_) => Domain::IPV6,
    };

    let protocol = match target {
        IpAddr::V4(_) => Protocol::ICMPV4,
        IpAddr::V6(_) => Protocol::ICMPV6,
    };

    let socket = Socket::new(domain, Type::from(SOCK_RAW), Some(protocol))?;

    socket.set_read_timeout(Some(Duration::from_millis(timeout as u64)))?;
    socket.set_write_timeout(Some(Duration::from_millis(timeout as u64)))?;

    if let IpAddr::V6(_) = target {
        socket.set_ttl(ttl as u32)?;
    } else {
        socket.set_multicast_ttl_v4(ttl as u32)?;
    }

    Ok(socket)
}

fn create_icmp_packet(payload_size: usize, target: IpAddr) -> Vec<u8> {
    let mut packet = vec![0u8; 8 + payload_size];

    match target {
        IpAddr::V4(_) => {
            packet[0] = 8; // ICMP Type: Echo Request (IPv4)
            packet[1] = 0; // Code: 0
        }
        IpAddr::V6(_) => {
            packet[0] = 128; // ICMPv6 Type: Echo Request
            packet[1] = 0; // Code: 0
        }
    }

    packet[2] = 0; // Checksum (initially 0, will be calculated)
    packet[3] = 0;
    packet[4] = 0; // Identifier
    packet[5] = 1;
    packet[6] = 0;
    packet[7] = 1;

    let mut rng = rand::thread_rng();
    rng.fill(&mut packet[8..]);

    let checksum = compute_checksum(&packet);
    packet[2] = (checksum >> 8) as u8;
    packet[3] = (checksum & 0xFF) as u8;

    packet
}

fn compute_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(2);

    for chunk in &mut chunks {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]);
        sum += word as u32;
    }

    if let Some(&[last_byte]) = chunks.remainder().get(0..1) {
        sum += ((last_byte as u16) << 8) as u32;
    }

    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

fn send_and_receive_ring(socket: &Socket, packet: &[u8], dest_addr: &SocketAddr, _timeout: i32) -> io::Result<Duration> {
    let start = Instant::now();
    let sockaddr = socket2::SockAddr::from(*dest_addr);
    socket.send_to(packet, &sockaddr)?;

    let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 1024];
    let read_size = socket.recv(&mut buffer)?;

    let _received_data = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr() as *const u8, read_size)
    };

    Ok(start.elapsed())
}


fn resolve_target(target: &str) -> Result<IpAddr, String> {
    match (target, 0).to_socket_addrs() {
        Ok(iter) => {
            let mut ipv4_addr = None;
            let mut ipv6_addr = None;

            for addr in iter {
                match addr.ip() {
                    IpAddr::V4(ipv4) => ipv4_addr = Some(IpAddr::V4(ipv4)),
                    IpAddr::V6(ipv6) => ipv6_addr = Some(IpAddr::V6(ipv6)),
                }
            }

            if let Some(ipv4) = ipv4_addr {
                return Ok(ipv4);
            }
            if let Some(ipv6) = ipv6_addr {
                return Ok(ipv6);
            }

            Err("No valid IP address found.".to_string())
        }
        Err(e) => Err(format!("Failed to resolve domain: {}", e)),
    }
}
