/// Listen address for the native binary.
///
/// Defaults stay `0.0.0.0:8787` so `npm run dev --prefix demo` and the Vite
/// `/api` proxy keep working. `demo/wasmer.toml` sets `PORT=80` and
/// `BIND_ADDR=127.0.0.1` for the Edge package command.
pub fn listen_addr() -> String {
    format_listen_addr(
        std::env::var("BIND_ADDR").ok().as_deref(),
        std::env::var("PORT").ok().as_deref(),
    )
}

pub fn format_listen_addr(bind: Option<&str>, port: Option<&str>) -> String {
    let bind = bind
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("0.0.0.0");
    let port = port
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787);
    format!("{bind}:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_vite_proxy() {
        assert_eq!(format_listen_addr(None, None), "0.0.0.0:8787");
        assert_eq!(format_listen_addr(Some(""), Some("")), "0.0.0.0:8787");
    }

    #[test]
    fn wasmer_package_env() {
        assert_eq!(
            format_listen_addr(Some("127.0.0.1"), Some("80")),
            "127.0.0.1:80"
        );
    }

    #[test]
    fn invalid_port_falls_back() {
        assert_eq!(
            format_listen_addr(Some("0.0.0.0"), Some("nope")),
            "0.0.0.0:8787"
        );
    }
}
