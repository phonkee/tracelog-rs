#![allow(unused_imports)]

mod info;
mod options;

pub use options::*;

use hyper::{Body, Request as HyperRequest, Response as HyperResponse};
use std::fmt::Debug;
use tonic::body::BoxBody;
use tonic::codegen::{Context, Poll, Service};
use tonic::transport::NamedService;
use tracing::{event, info};
use tracing::{span, Level};
use uuid::Uuid;

/// intercept service
pub fn intercept<S>(t: S, options: Options) -> InterceptedService<S> {
    InterceptedService {
        inner: t,
        options,
        funcs: vec![],
    }
}

// Fun, is it?
type Fun = fn(HyperRequest<Body>) -> HyperRequest<Body>;

#[derive(Debug, Clone)]
pub struct InterceptedService<S> {
    inner: S,
    options: Options,
    funcs: Vec<Fun>,
}

impl<Body> InterceptedService<Body> {
    pub fn with(mut self, fun: Fun) -> Self {
        self.funcs.push(fun);
        self
    }
}

impl<S> Service<HyperRequest<Body>> for InterceptedService<S>
where
    S: Service<HyperRequest<Body>, Response = HyperResponse<BoxBody>>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    // hello my friend, may I call you?
    fn call(&mut self, req: HyperRequest<Body>) -> Self::Future {
        let mut svc = self.inner.clone();

        // TODO: think about not cloning...
        let options = self.options.clone();

        let _h = req.headers();

        // do mutable request
        let mut req = HyperRequest::from(req);
        let ext = req.extensions_mut();
        ext.insert(32u32);

        // run all interceptors
        for fun in &self.funcs {
            req = fun(req)
        }

        Box::pin(async move {
            // first get start time
            let start = std::time::Instant::now();

            // get service and method
            let call_info = parse_path(req.uri().path().to_string());

            // get trace_id (or create new one)
            // let trace_id = extract(&req, &options.header);

            // change span for this request
            //            let span = span!(Level::INFO, "", "trace_id={:?}", trace_id);

            // enter it
            //            let _guard = span.enter();

            // call inner service
            let result = svc.call(req).await;

            let duration = format!("{:?}", std::time::Instant::now().duration_since(start));
            let duration: &str = &duration;

            // check if we want verbose names
            if !options.verbose_name {
                // todo
            }

            event!(
                Level::INFO,
                "\"request finished\" service={:?} method={:?} duration={:?}",
                call_info.service,
                call_info.method,
                duration
            );

            result
        })
    }
}

impl<S: NamedService> NamedService for InterceptedService<S> {
    const NAME: &'static str = S::NAME;
}

/// parse path and extract service and method
fn parse_path(path: String) -> info::Info {
    let splitted: Vec<&str> = path.strip_prefix("/").unwrap().splitn(2, "/").collect();
    if splitted.len() < 2 {
        return info::Info {
            service: splitted[0].to_string(),
            method: "".to_string(),
        };
    }
    info::Info {
        service: splitted[0].to_string(),
        method: splitted[1].to_string(),
    }
}

#[allow(dead_code)]
fn to_simple_service_name(name: &str) -> &str {
    let splitted: Vec<&str> = name.rsplitn(2, ".").collect();
    if let Some(l) = splitted.first() {
        l
    } else {
        name
    }
}

// extract trace_id from request
#[allow(dead_code)]
fn extract(_req: &HyperRequest<Body>, header: &Header) -> String {
    // TODO: this is dangerous operation now, please fix it future me
    // TODO: next clone?
    _req.headers()
        .get(header.0.clone())
        .map(|v| v.to_str().unwrap().into())
        .unwrap_or(Uuid::new_v4().to_string())
}
