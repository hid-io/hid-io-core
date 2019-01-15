extern crate capnp;
extern crate capnp_rpc;
extern crate hid_io;
extern crate tokio;

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::io::AsyncRead;
use tokio::prelude::Future;

use hid_io::hidio_capnp::h_i_d_i_o_server;

pub fn main() {
    try_main().unwrap();
}

fn try_main() -> Result<(), ::capnp::Error> {
    use std::net::ToSocketAddrs;
    let addr = "127.0.0.1:7185"
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let mut runtime = ::tokio::runtime::current_thread::Runtime::new().unwrap();
    let stream = runtime
        .block_on(::tokio::net::TcpStream::connect(&addr))
        .unwrap();
    stream.set_nodelay(true)?;
    let (reader, writer) = stream.split();

    let network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let hidio_server: h_i_d_i_o_server::Client =
        rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    runtime.spawn(rpc_system.map_err(|_e| ()));

    {
        let hidio = {
            let mut request = hidio_server.basic_request();
            request.send().pipeline.get_port()
        };

        let mut req = hidio.signal_request();
        req.get().set_time(27);
        runtime.block_on(req.send().promise.and_then(|response| {
            println!("RESP {}", pry!(response.get()).get_time());
            Promise::ok(())
        }))?;
    }

    Ok(())
}
