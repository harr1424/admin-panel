use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures_util::future::{ok, Ready};
use futures_util::FutureExt;
use std::task::{Context, Poll};

pub struct SecurityHeaders;

impl<S, B> actix_service::Transform<S, ServiceRequest> for SecurityHeaders
where
    S: actix_service::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SecurityHeadersMiddleware { service })
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: S,
}

impl<S, B> actix_service::Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: actix_service::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =
        futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        async move {
            let mut res = fut.await?;

            res.headers_mut().insert(
                actix_web::http::header::CONTENT_SECURITY_POLICY,
                "default-src 'self'; \
                 script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; \
                 style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; \
                 img-src 'self' data: blob: https:; \
                 font-src 'self' https://cdn.jsdelivr.net; \
                 connect-src 'self'; \
                 frame-ancestors 'none'; \
                 form-action 'self'; \
                 base-uri 'self'; \
                 upgrade-insecure-requests; \
                 block-all-mixed-content"
                    .parse()
                    .unwrap(),
            );
            res.headers_mut().insert(
                actix_web::http::header::X_CONTENT_TYPE_OPTIONS,
                "nosniff".parse().unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::X_FRAME_OPTIONS,
                "DENY".parse().unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::STRICT_TRANSPORT_SECURITY,
                "max-age=31536000; includeSubDomains; preload"
                    .parse()
                    .unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::PERMISSIONS_POLICY,
                "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), \
                             microphone=(), payment=(), usb=()"
                    .parse()
                    .unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::REFERRER_POLICY,
                "strict-origin-when-cross-origin".parse().unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::CROSS_ORIGIN_OPENER_POLICY,
                "same-origin".parse().unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::CROSS_ORIGIN_RESOURCE_POLICY,
                "same-origin".parse().unwrap(),
            );

            res.headers_mut().insert(
                actix_web::http::header::CROSS_ORIGIN_EMBEDDER_POLICY,
                "require-corp".parse().unwrap(),
            );
            Ok(res)
        }
        .boxed_local()
    }
}
