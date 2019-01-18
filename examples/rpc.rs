use capnp;

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::io::AsyncRead;
use tokio::prelude::Future;

use hid_io::api::{load_certs, load_private_key};
use hid_io::hidio_capnp::h_i_d_i_o_server;

use std::fs;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::prelude::future::ok;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const USE_SSL: bool = false;

pub fn main() {
    try_main().unwrap();
}

fn try_main() -> Result<(), ::capnp::Error> {
    let host = "localhost";
    let addr = format!("{}:7185", host)
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let ssl_config = if USE_SSL {
        let mut pem = BufReader::new(fs::File::open("test-ca/rsa/ca.cert").unwrap());
        let mut config = ClientConfig::new();
        config.root_store.add_pem_file(&mut pem).unwrap();
        config.set_single_client_cert(
            load_certs("test-ca/rsa/client.cert"),
            load_private_key("test-ca/rsa/client.key"),
        );
        let config = TlsConnector::from(Arc::new(config));
        Some(config)
    } else {
        None
    };

    trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite {};
    impl<T> Duplex for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

    let socket = ::tokio::net::TcpStream::connect(&addr).and_then(|socket| {
        socket.set_nodelay(true).unwrap();
        let c: Box<Future<Item = Box<_>, Error = std::io::Error>> =
            if let Some(config) = &ssl_config {
                let domain = webpki::DNSNameRef::try_from_ascii_str(host).unwrap();
                Box::new(config.connect(domain, socket).and_then(|a| {
                    let accept: Box<Duplex> = Box::new(a);
                    ok(accept)
                }))
            } else {
                let accept: Box<Duplex> = Box::new(socket);
                Box::new(ok(accept))
            };

        c
    });

    let mut runtime = ::tokio::runtime::current_thread::Runtime::new().unwrap();
    let stream = runtime.block_on(socket).unwrap();
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
            let request = hidio_server.basic_request();
            request.send().pipeline.get_port()
        };

        let mut req = hidio.signal_request();
        req.get().set_time(27);
        runtime.block_on(req.send().promise.and_then(|response| {
            println!("RESP {}", pry!(response.get()).get_time());
            Promise::ok(())
        }))?;

        let mut req = hidio.nodes_request();
        runtime.block_on(req.send().promise.and_then(|response| {
            let nodes = pry!(pry!(response.get()).get_nodes());
            for n in nodes.iter() {
                println!("Node {}", n.get_name().unwrap_or(""));
            }
            Promise::ok(())
        }))?;
    }

    Ok(())
}
