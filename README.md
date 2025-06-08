# Simple Rust + Local LLM Tool Calling

A Rust-based chat server with tool calling capabilities, including web search functionality.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- Cargo (comes with Rust)

## Getting Started

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/5aikat213/localllm-rust-tool-calling.git
   cd Simple-Tool-Calling
   ```

2. Build the project:
   ```
   cargo build
   ```

### Running the Server

Run the server using:
```
cargo run
```

The server will be available at http://127.0.0.1:8080

## API Endpoints

### Chat Endpoint
- **URL**: `/chat`
- **Method**: `POST`
- **Request Body**:
  ```json
  {
    "message": "Your message here"
  }
  ```

### Web Search
- **URL**: `/search`
- **Method**: `POST`
- **Request Body**:
  ```json
  {
    "query": "Your search query",
    "count": 5  // Optional, default is 5
  }
  ```

## Development

To run the server in development mode with logging:
```
RUST_LOG=debug cargo run
``` 