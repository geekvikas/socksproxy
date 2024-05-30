#[cfg(test)]
mod tests {
    use socksproxy::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    // Define constants
    const SOCKS_VERSION: u8 = 0x05;
    const RESERVED: u8 = 0x00;

    async fn get_free_address() -> String {
        return TcpListener::bind("0.0.0.0:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap()
            .to_string();
    }

    async fn create_test_client(addr: &str) -> TcpStream {
        TcpStream::connect(addr).await.unwrap()
    }

    async fn start_proxy(port: u16) {
        let mut proxy = SocksProxy::new(
            port,
            "0.0.0.0",
            vec![AuthMethods::NoAuth as u8],
            Option::Some(Duration::from_secs(1)),
        )
        .await
        .unwrap();

        tokio::spawn(async move {
            proxy.serve().await;
        });
    }

    async fn init_client(addr: &str) -> TcpStream {
        let mut client = create_test_client(addr).await;
        client
            .write_all(&[SOCKS_VERSION, 1, AuthMethods::NoAuth as u8])
            .await
            .unwrap();
        let mut buf = [0u8; 2];
        client.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf[0], SOCKS_VERSION);
        assert_eq!(buf[1], AuthMethods::NoAuth as u8);
        client
    }

    #[tokio::test]
    async fn test_socks_proxy_new() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();
        let proxy = SocksProxy::new(
            port,
            "0.0.0.0",
            vec![AuthMethods::NoAuth as u8],
            Option::Some(Duration::from_secs(1)),
        )
        .await;
        assert!(proxy.is_ok());
    }

    #[tokio::test]
    async fn test_socks_client_init_no_auth() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let _client = init_client(&addr).await;
    }

    #[tokio::test]
    async fn test_socks_client_handle_connect_ipv4() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let mut client = init_client(&addr).await;

        // Send SOCKS5 connect request
        client
            .write_all(&[
                SOCKS_VERSION,
                SockCommand::Connect as u8,
                RESERVED,
                AddrType::V4 as u8,
                1,
                1,
                1,
                1,
                0,
                80,
            ])
            .await
            .unwrap();
        let mut reply = [0u8; 10];
        client.read_exact(&mut reply).await.unwrap();

        assert_eq!(reply[0], SOCKS_VERSION);
        assert_eq!(reply[1], ResponseCode::Success as u8);
    }

    #[tokio::test]
    async fn test_socks_client_handle_connect_ipv6() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let mut client = init_client(&addr).await;

        // Send SOCKS5 connect request for IPv6
        client
            .write_all(&[
                SOCKS_VERSION,
                SockCommand::Connect as u8,
                RESERVED,
                AddrType::V6 as u8,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                255,
                255,
                1,
                1,
                1,
                1,
                0,
                80,
            ])
            .await
            .unwrap();
        let mut reply = [0u8; 10];
        client.read_exact(&mut reply).await.unwrap();

        assert_eq!(reply[0], SOCKS_VERSION);
        assert_eq!(reply[1], ResponseCode::Success as u8);
    }

    #[tokio::test]
    async fn test_socks_client_handle_connect_domain() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let mut client = init_client(&addr).await;

        // Send SOCKS5 connect request for domain name
        let domain = b"google.com";
        let domain_len = domain.len() as u8;
        let mut request = vec![
            SOCKS_VERSION,
            SockCommand::Connect as u8,
            RESERVED,
            AddrType::Domain as u8,
            domain_len,
        ];
        request.extend_from_slice(domain);
        request.extend_from_slice(&[0, 80]);
        client.write_all(&request).await.unwrap();
        let mut reply = [0u8; 10];
        client.read_exact(&mut reply).await.unwrap();

        assert_eq!(reply[0], SOCKS_VERSION);
        assert_eq!(reply[1], ResponseCode::Success as u8);
    }

    #[tokio::test]
    async fn test_socks_client_handle_invalid_version() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let mut client = create_test_client(&addr).await;
        client
            .write_all(&[0x04, 1, AuthMethods::NoAuth as u8])
            .await
            .unwrap();
        let mut buf = [0u8; 2];
        let result = client.read_exact(&mut buf).await;

        assert!(result.is_err()); // Expect an error due to invalid SOCKS version
    }

    #[tokio::test]
    async fn test_socks_client_handle_invalid_command() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let mut client = init_client(&addr).await;

        // Send invalid SOCKS5 command
        client
            .write_all(&[
                SOCKS_VERSION,
                0x09,
                RESERVED,
                AddrType::V4 as u8,
                127,
                0,
                0,
                1,
                0,
                80,
            ])
            .await
            .unwrap();
        let mut reply = [0u8; 10];
        assert!(client.read_exact(&mut reply).await.is_err());
    }

    #[tokio::test]
    async fn test_socks_client_handle_no_auth_methods() {
        let addr = get_free_address().await;
        let port = addr.split(':').collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap();

        start_proxy(port).await;
        let mut client = init_client(&addr).await;

        client.write_all(&[SOCKS_VERSION, 1, 0x05]).await.unwrap(); // Send unsupported auth method
        let mut buf = [0u8; 2];

        assert!(client.try_read(&mut buf).is_err());
    }
}
