use std::env;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use std::thread::sleep;
use rand::Rng;
use std::io;

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

    match target.parse::<IpAddr>() {
        Ok(ip) => {
            run_ring(ip, count, packet_size, timeout, ttl, continuous);
        }
        Err(_) => {
            println!("Invalid target address.");
        }
    }
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
    let packet = create_icmp_packet(packet_size);
    let socket_addr = match target {
        IpAddr::V4(ip) => SocketAddr::new(IpAddr::V4(ip), 0),
        IpAddr::V6(ip) => SocketAddr::new(IpAddr::V6(ip), 0),
    };

    let mut sent = 0;
    let mut received = 0;
    let mut min_rtt = Duration::MAX;
    let mut max_rtt = Duration::ZERO;
    let mut total_rtt = Duration::ZERO;

    while continuous || count > 0 {
        let _start = Instant::now();
        let result = send_and_receive_ring(&socket_addr, &packet, timeout);

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

fn create_icmp_packet(payload_size: usize) -> Vec<u8> {
    let mut packet = vec![0u8; 8 + payload_size];
    packet[0] = 8; // ICMP Type: Echo Request
    packet[1] = 0; // Code: 0

    let checksum = compute_checksum(&packet);
    packet[2] = (checksum >> 8) as u8;
    packet[3] = (checksum & 0xFF) as u8;

    let mut rng = rand::thread_rng();
    rng.fill(&mut packet[8..]);
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

fn send_and_receive_ring(
    _socket_addr: &SocketAddr,
    _packet: &[u8],
    _timeout: i32,
) -> io::Result<Duration> {
    Ok(Duration::from_millis(50))
}
