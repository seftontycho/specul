# Specul is a simple, asynchronos rcon client for rust.
See https://developer.valvesoftware.com/wiki/Source_RCON_Protocol.
<br></br>
# Examples
```rust
use specul::Connection;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
  let tcp = TcpStream::connect(0.0.0.0:8080).await?;
  let cnn = Connection::<TcpStream>::builder().build(io);
}
```
