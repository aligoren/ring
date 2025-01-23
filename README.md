# 🏓 Ring - A Modern Alternative to `ping`

Ring is a **high-performance, Rust-based** alternative to the traditional `ping` command. It allows users to **send ICMP Echo Request packets** to network hosts and measure their response times, providing insights into network latency and packet loss. Unlike traditional `ping`, **Ring** is designed to be **lightweight, efficient, and highly configurable**, leveraging Rust's safety guarantees and performance optimizations. It supports **IPv4 & IPv6**, adjustable **packet size**, **TTL settings**, **custom timeout values**, and much more.

Ring operates using **ICMP (Internet Control Message Protocol)**, which is used to determine the latency and reachability of a networked host. The tool follows standard networking protocols, specifically:
- **[RFC 792 - Internet Control Message Protocol (ICMP)](https://tools.ietf.org/html/rfc792)**
- **[RFC 1122 - Host Requirements](https://tools.ietf.org/html/rfc1122)**
- **[RFC 4443 - ICMPv6](https://tools.ietf.org/html/rfc4443)** (For IPv6 support)

## 🔧 Installation & Usage

To build Ring manually using **git**, run:

```sh
git clone https://github.com/aligoren/ring.git
cd ring && cargo build --release
```

After installation, you can start using Ring just like `ping`:

`ring 8.8.8.8`

This will send ICMP Echo Requests to Google’s public DNS server with default settings. If you need more control, you can use various options:

`ring <target> [options]`

| Option     | Description               | Example                 |
|------------|---------------------------|-------------------------|
| `-c <n>`   | Number of packets to send | `ring 8.8.8.8 -c 5`     |
| `-s <n>`   | Packet size (bytes)       | `ring 8.8.8.8 -s 64`    |
| `-w <ms>`  | Timeout in milliseconds   | `ring 8.8.8.8 -w 1000`  |
| `-ttl <n>` | Set Time-to-Live (TTL)    | `ring 8.8.8.8 -ttl 128` |
| `-t`       | Continuous ping mode      | `ring 8.8.8.8 -t`       |
| `-4`       | Force IPv4 mode           | `ring example.com -4`   |
| `-6`       | Force IPv6 mode           | `ring example.com -6`   |


Each ping operation follows a simple request-response model:

```text
+------------+     ICMP Echo Request      +------------+
|  Ring CLI  |  -----------------------> | Target Host |
|  (User)    |                           | (e.g., 8.8.8.8) |
+------------+     ICMP Echo Reply       +------------+
       |     <-----------------------
       |        Calculates RTT, Packet Loss, and Stats
       v
  Displays Results
```

## 🛠️ Development & Testing

Ring is built using Rust and requires administrator (root) privileges to access raw sockets. To run the program in debug mode, use:

`cargo run -- 8.8.8.8 -c 3 -s 32`

To execute the test suite:

`cargo test`

For production builds:

`cargo build --release`

## 🎯 Future Enhancements & Contribution

We plan to introduce the following features:

📡 Parallel pinging of multiple hosts

📜 JSON output support for automation

📈 Graphical statistics

🖥️ Web-based interface for real-time monitoring

Contributions are welcome! If you would like to contribute, feel free to open issues or submit pull requests.

## 📄 License

This project is licensed under the MIT License.
