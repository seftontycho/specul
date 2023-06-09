# Specul is a simple, asynchronous rcon client for rust.
See https://developer.valvesoftware.com/wiki/Source_RCON_Protocol.
Tested to work with Factorio.

# Examples
```rust
use specul::{Connection, ConnectionBuilder};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let tcp = TcpStream::connect(0.0.0.0:8080).await?;
    let cnn = Connection::default().io(tcp).build()?;
    
    cnn.authenticate("password").await?;
    let responses = cnn.execute_command("Hello World!").await?;
    println!("Responses: {}", responses);
}

```
