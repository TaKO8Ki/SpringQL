use std::{
    io::{self, BufRead, BufReader},
    net::{IpAddr, SocketAddr, TcpStream},
    time::Duration,
};

use anyhow::Context;

use crate::{
    error::{Result, SpringError},
    stream_engine::{
        executor::foreign_input_row::{format::json::JsonObject, ForeignInputRow},
        model::option::Options,
    },
};

use super::{InputServerActive, InputServerStandby};

#[derive(Debug)]
enum Protocol {
    Tcp,
}

// TODO config
const CONNECT_TIMEOUT_SECS: u64 = 1;
const READ_TIMEOUT_MSECS: u64 = 100;

#[derive(Debug)]
pub(in crate::stream_engine::executor) struct NetInputServerStandby {
    protocol: Protocol,
    remote_host: IpAddr,
    remote_port: u16,
}

#[derive(Debug)]
pub(in crate::stream_engine::executor) struct NetInputServerActive {
    foreign_addr: SocketAddr,
    tcp_stream_reader: BufReader<TcpStream>, // TODO UDP
}

impl InputServerStandby<NetInputServerActive> for NetInputServerStandby {
    fn new(options: Options) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            protocol: options.get("PROTOCOL", |protocol_str| {
                (protocol_str == "TCP")
                    .then(|| Protocol::Tcp)
                    .context("unsupported protocol")
            })?,
            remote_host: options.get("REMOTE_HOST", |remote_host_str| {
                remote_host_str.parse().context("invalid remote host")
            })?,
            remote_port: options.get("REMOTE_PORT", |remote_port_str| {
                remote_port_str.parse().context("invalid remote port")
            })?,
        })
    }

    fn start(self) -> Result<NetInputServerActive> {
        let sock_addr = SocketAddr::new(self.remote_host, self.remote_port);

        let tcp_stream =
            TcpStream::connect_timeout(&sock_addr, Duration::from_secs(CONNECT_TIMEOUT_SECS))
                .context("failed to connect to remote host")
                .map_err(|e| SpringError::ForeignIo {
                    source: e,
                    foreign_info: format!("{:?}", sock_addr),
                })?;
        tcp_stream
            .set_read_timeout(Some(Duration::from_millis(READ_TIMEOUT_MSECS)))
            .context("failed to set timeout to remote host")
            .map_err(|e| SpringError::ForeignIo {
                source: e,
                foreign_info: format!("{:?}", sock_addr),
            })?;

        let tcp_stream_reader = BufReader::new(tcp_stream);

        Ok(NetInputServerActive {
            tcp_stream_reader,
            foreign_addr: sock_addr,
        })
    }
}

impl InputServerActive for NetInputServerActive {
    /// # Failure
    ///
    /// - [SpringError::ForeignInputTimeout](crate::error::SpringError::ForeignInputTimeout) when:
    ///   - Remote source does not provide row within timeout.
    fn next_row(&mut self) -> Result<ForeignInputRow> {
        let mut json_s = String::new();

        self.tcp_stream_reader
            .read_line(&mut json_s)
            .map_err(|io_err| {
                if let io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock = io_err.kind() {
                    SpringError::ForeignInputTimeout {
                        source: anyhow::Error::from(io_err),
                        foreign_info: format!("{:?}", self.foreign_addr),
                    }
                } else {
                    SpringError::ForeignIo {
                        source: anyhow::Error::from(io_err),
                        foreign_info: format!("{:?}", self.foreign_addr),
                    }
                }
            })?;

        self.parse_resp(&json_s)
    }
}

impl NetInputServerActive {
    fn parse_resp(&self, json_s: &str) -> Result<ForeignInputRow> {
        let json_v = serde_json::from_str(json_s)
            .with_context(|| {
                format!(
                    r#"failed to parse message from foreign stream as JSON ("{}")"#,
                    json_s,
                )
            })
            .map_err(|e| SpringError::ForeignIo {
                source: e,
                foreign_info: format!("{:?}", self.foreign_addr),
            })?;

        let json_obj = JsonObject::new(json_v);

        Ok(ForeignInputRow::from_json(json_obj))
    }
}

impl NetInputServerActive {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_engine::executor::foreign_input_row::format::json::JsonObject;
    use crate::stream_engine::executor::foreign_input_row::ForeignInputRow;
    use crate::stream_engine::executor::test_support::foreign::source::TestSource;
    use crate::stream_engine::model::option::options_builder::OptionsBuilder;
    use crate::timestamp::Timestamp;

    #[test]
    fn test_input_server_tcp() -> crate::error::Result<()> {
        let j1 = JsonObject::fx_tokyo(Timestamp::fx_ts1());
        let j2 = JsonObject::fx_osaka(Timestamp::fx_ts2());
        let j3 = JsonObject::fx_london(Timestamp::fx_ts3());

        let source = TestSource::start(vec![j2.clone(), j3.clone(), j1.clone()])?;

        let options = OptionsBuilder::default()
            .add("PROTOCOL", "TCP")
            .add("REMOTE_HOST", "127.0.0.1")
            .add("REMOTE_PORT", source.port().to_string())
            .build();

        let server = NetInputServerStandby::new(options)?;
        let mut server = server.start()?;

        assert_eq!(server.next_row()?, ForeignInputRow::from_json(j2));
        assert_eq!(server.next_row()?, ForeignInputRow::from_json(j3));
        assert_eq!(server.next_row()?, ForeignInputRow::from_json(j1));
        assert!(matches!(
            server.next_row().unwrap_err(),
            SpringError::ForeignInputTimeout { .. }
        ));

        Ok(())
    }
}
