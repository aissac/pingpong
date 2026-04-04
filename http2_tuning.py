# Implement HTTP/2 tuning and connection pre-warming

with open('/home/ubuntu/polymarket-hft-engine/src/api.rs', 'r') as f:
    content = f.read()

# Find the Client::new() and replace with optimized HTTP/2 client
old_client = '''        use reqwest::Client;
        
        let client = Client::new();'''

new_client = '''        use reqwest::Client;
        
        // HTTP/2 tuning for Polymarket API (NotebookLM recommendation)
        // 512KB stream windows for ~469KB WebSocket payloads
        let client = Client::builder()
            .http2_prior_knowledge()
            .http2_initial_stream_window_size(512 * 1024)
            .http2_initial_connection_window_size(512 * 1024)
            .pool_max_idle_per_host(20)  // Connection pre-warming
            .pool_idle_timeout(std::time::Duration::from_secs(300))
            .tcp_nodelay(true)  // Disable Nagle's algorithm
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            .build()
            .expect("Failed to create HTTP/2 client");'''

content = content.replace(old_client, new_client)

# Find the second Client::new() and replace similarly
content = content.replace(
    '''        use reqwest::Client;
        let client = Client::new();''',
    new_client
)

with open('/home/ubuntu/polymarket-hft-engine/src/api.rs', 'w') as f:
    f.write(content)

print('Added HTTP/2 tuning and connection pre-warming')