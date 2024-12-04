# RPC Proxy Service

A service that listens for HTTP-RPC requests and forwards the request body to remote servers according to the configured rules.

## Running

1. Clone the repo:
    ```bash
    git clone https://github.com/dexterlaboss/rpc-proxy
    cd rpc-proxy
    ```

2. Build and run:
    ```bash
    cargo run
    ```

3. Test the service:
    ```bash
    curl -X POST http://localhost:8889 -d '{"key":"value"}' -H "Content-Type: application/json"
    ```

