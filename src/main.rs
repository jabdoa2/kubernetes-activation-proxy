use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use std::{convert::Infallible, net::SocketAddr};
use std::env;
use hyper_reverse_proxy::ProxyError;

async fn handle(request: Request<Body>, addr: SocketAddr, activation_server: String) -> Result<Response<Body>, Infallible> {
    let host = match request.headers().get("host") {
        Some(value) => match value.to_str() {
            Ok(host_str) => host_str,
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Cannot convert host"))
                    .unwrap());
            }
        },
        None => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Missing host header"))
                .unwrap());
        }
    };

    println!(
        "Request: {} {} {} {}",
        request.method(),
        host,
        request.uri(),
        addr
    );
    let autoscaler_client = Client::new();
    let autoscaler_response = autoscaler_client
        .request(
            Request::builder()
                .method(Method::POST)
                .uri(activation_server.to_owned())
                .body(Body::from(host.to_owned()))
                .unwrap(),
        )
        .await;



    let upstream_url = match autoscaler_response {
        Ok(upstream_response) => {
            if upstream_response.status() != 200 {
                println!("Activation server response code: {}", upstream_response.status());
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Cannot reach actication server."))
                    .unwrap());
            }
            match hyper::body::to_bytes(upstream_response.into_body()).await {
                Ok(url) => match String::from_utf8(url.to_vec()) {
                    Ok(url) => url,
                    Err(error) => {
                        println!("Error in encoding from activation server: {}", error);
                        return Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("Cannot reach actication server."))
                            .unwrap());
                    }
                },
                Err(error) => {
                    println!("Error from activation server: {}", error);
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Cannot reach actication server."))
                        .unwrap());
                }
            }
        },
        Err(error) => {
            println!("Error from activation server: {}", error);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Cannot reach actication server."))
                .unwrap());
        }
    };

    let response = match hyper_reverse_proxy::call(addr.ip(), &upstream_url, request).await {
        Ok(request) => request,
        Err(ProxyError::InvalidUri(error)) => {
            println!("Invalid URL from actication server: {}", error);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Cannot reach actication server."))
                .unwrap())
        }
        Err(ProxyError::ForwardHeaderError) => {
            println!("Error with forward header");
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::empty())
                .unwrap()
        }
        Err(ProxyError::HyperError(error)) => {
            println!("Error from upstream after activation: {}", error);
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("Upstream cannot be reached after activation."))
                .unwrap()
        }
    };
    Ok(response)
}

#[tokio::main]
async fn main() {
    let bind_addr = env::var("BIND_HOST_PORT").unwrap_or("0.0.0.0:8000".into());
    let activation_server = env::var("ACTIVATION_SERVER").expect("Please specify ACTIVATION_SERVER.");

    let addr:SocketAddr = bind_addr.parse().expect("Could not parse ip:port.");

    let make_svc = make_service_fn(|conn: &AddrStream| {
        let addr = conn.remote_addr();
        let activation_server = activation_server.to_owned();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| handle(req, addr, activation_server.clone())))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
