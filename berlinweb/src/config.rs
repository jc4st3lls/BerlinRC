//! Configuration loader and defaults for the berlinweb server.
//!
//! Exposes a lazily-initialized `CONFIG` which reads values from environment
//! variables (with sensible defaults). Fields include authentication
//! settings (`password`, `otp_secret`) and TLS assets (`cert`, `key`), plus
//! listening ports (`web_port`, `hub_port`).
//!
use std::env;

use base64::{Engine as _, engine::general_purpose};
use berlinproto::otp::generate_otp_secret;
use once_cell::sync::Lazy;

/// Default password for authentication
const DEFAULT_PASSWORD: &str = "12345678";

const DEFAULT_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIC7DCCAdSgAwIBAgIQX/mmkaVZi4lBSkSMqM+TNDANBgkqhkiG9w0BAQsFADAU
MRIwEAYDVQQDEwlsb2NhbGhvc3QwHhcNMjIwNjE1MDgwNzQ3WhcNMjcwNjE1MDAw
MDAwWjAUMRIwEAYDVQQDEwlsb2NhbGhvc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IB
DwAwggEKAoIBAQC1uejE09rrdmbAXcMXQW4iT1Uj090qK3bTZVpT4BfY5Ci35wbW
leKvTXrVohcJBkcJdeUoIyWQRgdQdHhILBr0evam5bwT2QuCVvCJJay7Oo2+M9wW
y+waIUoicLFifQZvEKJRfvJGsfsNvlX9HL6uU6+VQhBYd8ytFSeuECFU/YtsYr/H
cLsxFiNriFcP0Q4eoxTn6QHrUmQBDI/kAswncfW9Wt0fbem5tbuUWNmAWyNw0BAh
M6ENbmhPsCp/lFBcJ0AT5CPaAZgwUh6wlCPzwlXa15rBFdf3zFDxb1fiZHnWXidk
uQ0VRL8kZCuD0kO1lQoU38hCoZRYuK1YJO45AgMBAAGjOjA4MAsGA1UdDwQEAwIE
sDATBgNVHSUEDDAKBggrBgEFBQcDATAUBgNVHREEDTALgglsb2NhbGhvc3QwDQYJ
KoZIhvcNAQELBQADggEBAEj4X8jRsnS+qF+dSv2y5aKCwwWneXr8fASq4VlFLg/X
XBlrlDP1rK3EsGf71Y4L+IMOvxlDB3f5m7jHrLOungk90tBbiikvUsBVfhTsWUtV
79SOi58r+YmQza0zsN9uTmvpLkKd/bRhTX0BS1Pcno+MYUWr+Bqrn11Ubvxob2SY
5sfqd8YmS0glU5UunL/JKmNQwmOpNUA7VzlLazNJb3td1U8fzN0CvITykxKS+Zt5
qD813jTP8879eewxXqmF2tNYy8CDW8ckQNatzAQIdJxjdnlTh5HjhxPGFwz4rBI8
5n9NbvgWzWHv0dzSGDuKiN1gJ6HRLm3QsX/Hb3i3VsM=
-----END CERTIFICATE-----";

/// Default SSL/TLS private key for HTTPS
const DEFAULT_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC1uejE09rrdmbA
XcMXQW4iT1Uj090qK3bTZVpT4BfY5Ci35wbWleKvTXrVohcJBkcJdeUoIyWQRgdQ
dHhILBr0evam5bwT2QuCVvCJJay7Oo2+M9wWy+waIUoicLFifQZvEKJRfvJGsfsN
vlX9HL6uU6+VQhBYd8ytFSeuECFU/YtsYr/HcLsxFiNriFcP0Q4eoxTn6QHrUmQB
DI/kAswncfW9Wt0fbem5tbuUWNmAWyNw0BAhM6ENbmhPsCp/lFBcJ0AT5CPaAZgw
Uh6wlCPzwlXa15rBFdf3zFDxb1fiZHnWXidkuQ0VRL8kZCuD0kO1lQoU38hCoZRY
uK1YJO45AgMBAAECggEABi+w+9pWboOWVeAbPxRsImDe/hw9QC1Am0us+oP7a9fA
hxonQnDRybPyhYlCDX2YN3s69NXVdobbwuJkIdjWhhIViXLypx5RZPt+rryIl8sT
fjEXwfLpM66Ebo21jCvDZ06CqBGRP9TZPguHs9khqJ+Sr5sTIV/aqN26fxNvfwwf
z/fYnI6HbhsSV4mdsIdWfbUr+W83zLHFKkjz6a5bbnC05DnU1nMjHQttrS82TgTg
XLCwCkduILBV3pp9AU6apeOXodgHphKvT5AxWBhlsysC7tc/X+l+LTz5EMU/KsPM
zHFOQmsy2DWvNz1hHrKZNlWxW22oYLjEslGgecblvQKBgQDFhanG5rh9J5qq1t2A
ADfiqkomDFqFZA5eWc+uveNoBFk+cWp+Rm9flcO/Q/TfUTr0tx4FJX/LXaoGpWrL
cmjWq9leFcrCPf1oeJJYHZllvhUe9gn2GcGqAN8eUOhBHOldQaLDq9g1fKLHtLNc
LRmNIuIF8nBIuqjqJKH8WMuWEwKBgQDrhxAc+hAGbUEg2CSs0Uml4lA/rz4FqSpV
vXwybn8xGRkFrSlHEBNb4Gl4DjHG3aJ9uIrUmNn/q5VFccZG3QVidAjcNIcjLOc5
5totlWs35B/zGGsqbhXco9UuS88K1h96pT5ZipUxoCUwIUAWW7AeFQ//El6JszbP
QbTWA6qkAwKBgQC/8kdtYbKw9PapxEnV5OBqJcAOv3yMGhKYf8CB+EfwQiGTu9WY
RsxeYASsbtac2axoOTc0Gx/YOfpLoR5p/JGC49dFRfoWzvTePCVC+eii5ZhS0RgX
DyqTEWvBYzCAbh8dn/YTHoDqYWcymRifn7gv3lE1JEcXdkVF3DmKJ6QX/wKBgGQy
9IbvV2v0hPWdHpUrAGMDEdLWEdPEsQ8C6thlq9TOcZe5oErsKuA2a4g4ubJ5zcwg
e2eQk4WykHGXwpuZIdZNuQs9iZRMYR5/+KfV3mRLt8/qvoSxirlwNZxZgf6BM6kw
rYLYczpGgCumqaYZYaaanVCNlwyL4rBvqqg1rR3TAoGBAIZZB8OCwZx1Az1I7x5t
I8Bkq6BFIHx9fogsU75mepGnYxcXx0m5u6UhT1YqnXm/HSpXjPfWIJERs+FJm67H
/eTNqR9sgC6pBfRT39nGWX6Ap5LYKxsXq29y476u3DeL4BZ4DUWBEBExG0h5y9RQ
FQKp3EYkGtce1TBY0rqQtgmM
-----END PRIVATE KEY-----";

const DEFAULT_WEB_PORT: u16 = 443;
const DEFAULT_HUB_PORT: u16 = 80;

/// Application configuration containing authentication and TLS settings
pub struct Config {
    /// Authentication password
    pub password: String,
    /// One-Time Password secret for 2FA
    pub otp_secret: String,
    /// SSL/TLS certificate
    pub cert: String,
    /// SSL/TLS private key
    pub key: String,
    /// Web https port
    pub web_port: u16,
    /// HUB tcp port
    pub hub_port: u16,
}

/// Global application configuration instance, lazily initialized
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let decode_maybe_b64 = |val: String| -> String {
        general_purpose::STANDARD
            .decode(&val)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or(val)
    };

    Config {
        password: env::var("BERLINRC_PASSWORD").unwrap_or_else(|_| DEFAULT_PASSWORD.into()),
        otp_secret: env::var("BERLINRC_OTP_SECRET").unwrap_or_else(|_| generate_otp_secret()),
        cert: decode_maybe_b64(env::var("BERLINRC_CERT").unwrap_or_else(|_| DEFAULT_CERT.into())),
        key: decode_maybe_b64(env::var("BERLINRC_KEY").unwrap_or_else(|_| DEFAULT_KEY.into())),
        web_port: env::var("BERLINRC_WEB_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_WEB_PORT),

        hub_port: env::var("BERLINRC_HUB_PORT") 
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_HUB_PORT),
    }
});
