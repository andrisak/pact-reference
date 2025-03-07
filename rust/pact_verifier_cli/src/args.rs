use clap::{App, Arg};
use regex::Regex;

fn port_value(v: &str) -> Result<(), String> {
  v.parse::<u16>().map(|_| ()).map_err(|e| format!("'{}' is not a valid port value: {}", v, e) )
}

fn integer_value(v: &str) -> Result<(), String> {
  v.parse::<u64>().map(|_| ()).map_err(|e| format!("'{}' is not a valid integer value: {}", v, e) )
}

fn transport_value(v: &str) -> Result<(String, u16), String> {
  let (transport, port) = v.split_once(':')
    .ok_or_else(|| format!("'{}' is not a valid transport, it must be in the form TRANSPORT:PORT", v))?;
  if transport.is_empty() {
    return Err(format!("'{}' is not a valid transport, the transport part is empty", v));
  }
  port.parse::<u16>().map(|port| (transport.to_string(), port))
    .map_err(|e| format!("'{}' is not a valid port value: {}", port, e) )
}

pub(crate) fn setup_app<'a>(program: &'a str, version: &'a str) -> App<'a> {
  App::new(program)
    .version(version)
    .about("Standalone Pact verifier")
    .version_short('v')
    .arg(Arg::with_name("loglevel")
      .short('l')
      .long("loglevel")
      .takes_value(true)
      .use_delimiter(false)
      .possible_values(&["error", "warn", "info", "debug", "trace", "none"])
      .help("Log level (defaults to warn)"))
    .arg(Arg::with_name("file")
      .short('f')
      .long("file")
      .required_unless_one(&["dir", "url", "broker-url"])
      .takes_value(true)
      .use_delimiter(false)
      .multiple(true)
      .number_of_values(1)
      .empty_values(false)
      .help("Pact file to verify (can be repeated)"))
    .arg(Arg::with_name("dir")
      .short('d')
      .long("dir")
      .required_unless_one(&["file", "url", "broker-url"])
      .takes_value(true)
      .use_delimiter(false)
      .multiple(true)
      .number_of_values(1)
      .empty_values(false)
      .help("Directory of pact files to verify (can be repeated)"))
    .arg(Arg::with_name("url")
      .short('u')
      .long("url")
      .required_unless_one(&["file", "dir", "broker-url"])
      .takes_value(true)
      .use_delimiter(false)
      .multiple(true)
      .number_of_values(1)
      .empty_values(false)
      .help("URL of pact file to verify (can be repeated)"))
    .arg(Arg::with_name("broker-url")
      .short('b')
      .long("broker-url")
      .env("PACT_BROKER_BASE_URL")
      .required_unless_one(&["file", "dir", "url"])
      .requires("provider-name")
      .takes_value(true)
      .use_delimiter(false)
      .empty_values(false)
      .help("URL of the pact broker to fetch pacts from to verify (requires the provider name parameter)"))
    .arg(Arg::with_name("hostname")
      .short('h')
      .long("hostname")
      .takes_value(true)
      .use_delimiter(false)
      .help("Provider hostname (defaults to localhost)"))
    .arg(Arg::with_name("port")
      .short('p')
      .long("port")
      .takes_value(true)
      .use_delimiter(false)
      .help("Provider port (defaults to protocol default 80/443)")
      .validator(port_value))
    .arg(Arg::with_name("transport")
      .long("transport")
      .alias("scheme")
      .takes_value(true)
      .default_value("http")
      .help("Provider protocol transport to use (http, https, grpc, etc.)"))
    .arg(Arg::with_name("transports")
      .long("transports")
      .takes_value(true)
      .multiple_values(true)
      .help("Allows multiple protocol transports to be configured (http, https, grpc, etc.) with their associated port numbers separated by a colon. For example, use --transports http:8080 grpc:5555 to configure both.")
      .value_parser(transport_value))
    .arg(Arg::with_name("provider-name")
      .short('n')
      .long("provider-name")
      .takes_value(true)
      .use_delimiter(false)
      .help("Provider name (defaults to provider)"))
    .arg(Arg::with_name("state-change-url")
      .short('s')
      .long("state-change-url")
      .takes_value(true)
      .use_delimiter(false)
      .help("URL to post state change requests to"))
    .arg(Arg::with_name("state-change-as-query")
      .long("state-change-as-query")
      .help("State change request data will be sent as query parameters instead of in the request body"))
    .arg(Arg::with_name("state-change-teardown")
      .long("state-change-teardown")
      .help("State change teardown requests are to be made after each interaction"))
    .arg(Arg::with_name("filter-description")
      .long("filter-description")
      .env("PACT_DESCRIPTION")
      .takes_value(true)
      .use_delimiter(false)
      .validator(|val| Regex::new(&val)
        .map(|_| ())
        .map_err(|err| format!("'{}' is an invalid filter value: {}", val, err)))
      .help("Only validate interactions whose descriptions match this filter"))
    .arg(Arg::with_name("filter-state")
      .long("filter-state")
      .env("PACT_PROVIDER_STATE")
      .takes_value(true)
      .use_delimiter(false)
      .conflicts_with("filter-no-state")
      .validator(|val| Regex::new(&val)
        .map(|_| ())
        .map_err(|err| format!("'{}' is an invalid filter value: {}", val, err)))
      .help("Only validate interactions whose provider states match this filter"))
    .arg(Arg::with_name("filter-no-state")
      .long("filter-no-state")
      .env("PACT_PROVIDER_NO_STATE")
      .conflicts_with("filter-state")
      .help("Only validate interactions that have no defined provider state"))
    .arg(Arg::with_name("filter-consumer")
      .short('c')
      .long("filter-consumer")
      .takes_value(true)
      .multiple(true)
      .empty_values(false)
      .help("Consumer name to filter the pacts to be verified (can be repeated)"))
    .arg(Arg::with_name("user")
      .long("user")
      .env("PACT_BROKER_USERNAME")
      .takes_value(true)
      .use_delimiter(false)
      .number_of_values(1)
      .empty_values(false)
      .conflicts_with("token")
      .help("Username to use when fetching pacts from URLS"))
    .arg(Arg::with_name("password")
      .long("password")
      .env("PACT_BROKER_PASSWORD")
      .takes_value(true)
      .use_delimiter(false)
      .number_of_values(1)
      .empty_values(false)
      .conflicts_with("token")
      .help("Password to use when fetching pacts from URLS"))
    .arg(Arg::with_name("token")
      .short('t')
      .long("token")
      .env("PACT_BROKER_TOKEN")
      .takes_value(true)
      .use_delimiter(false)
      .number_of_values(1)
      .empty_values(false)
      .conflicts_with("user")
      .help("Bearer token to use when fetching pacts from URLS"))
    .arg(Arg::with_name("publish")
      .long("publish")
      .requires("broker-url")
      .requires("provider-version")
      .help("Enables publishing of verification results back to the Pact Broker. Requires the broker-url and provider-version parameters."))
    .arg(Arg::with_name("provider-version")
      .long("provider-version")
      .takes_value(true)
      .use_delimiter(false)
      .number_of_values(1)
      .empty_values(false)
      .help("Provider version that is being verified. This is required when publishing results."))
    .arg(Arg::with_name("build-url")
      .long("build-url")
      .takes_value(true)
      .use_delimiter(false)
      .number_of_values(1)
      .empty_values(false)
      .help("URL of the build to associate with the published verification results."))
    .arg(Arg::with_name("provider-tags")
      .long("provider-tags")
      .takes_value(true)
      .use_delimiter(true)
      .empty_values(false)
      .help("Provider tags to use when publishing results. Accepts comma-separated values."))
    .arg(Arg::with_name("provider-branch")
      .long("provider-branch")
      .takes_value(true)
      .empty_values(false)
      .help("Provider branch to use when publishing results"))
    .arg(Arg::with_name("base-path")
      .long("base-path")
      .takes_value(true)
      .use_delimiter(false)
      .empty_values(false)
      .help("Base path to add to all requests"))
    .arg(Arg::with_name("consumer-version-tags")
      .long("consumer-version-tags")
      .takes_value(true)
      .use_delimiter(true)
      .empty_values(false)
      .requires("broker-url")
      .conflicts_with("consumer-version-selectors")
      .help("Consumer tags to use when fetching pacts from the Broker. Accepts comma-separated values."))
    .arg(Arg::with_name("consumer-version-selectors")
      .long("consumer-version-selectors")
      .takes_value(true)
      .use_delimiter(false)
      .multiple(true)
      .number_of_values(1)
      .empty_values(false)
      .requires("broker-url")
      .conflicts_with("consumer-version-tags")
      .help("Consumer version selectors to use when fetching pacts from the Broker. Accepts a JSON string as per https://docs.pact.io/pact_broker/advanced_topics/consumer_version_selectors/"))
    .arg(Arg::with_name("disable-ssl-verification")
      .long("disable-ssl-verification")
      .takes_value(false)
      .help("Disables validation of SSL certificates"))
    .arg(Arg::with_name("enable-pending")
      .long("enable-pending")
      .requires("broker-url")
      .help("Enables Pending Pacts"))
    .arg(Arg::with_name("include-wip-pacts-since")
      .long("include-wip-pacts-since")
      .takes_value(true)
      .use_delimiter(false)
      .number_of_values(1)
      .empty_values(false)
      .requires("broker-url")
      .help("Allow pacts that don't match given consumer selectors (or tags) to  be verified, without causing the overall task to fail. For more information, see https://pact.io/wip"))
    .arg(Arg::with_name("request-timeout")
      .long("request-timeout")
      .takes_value(true)
      .empty_values(false)
      .validator(integer_value)
      .help("Sets the HTTP request timeout in milliseconds for requests to the target API and for state change requests."))
    .arg(Arg::with_name("json-file")
      .short('j')
      .long("json")
      .takes_value(true)
      .use_delimiter(false)
      .multiple(false)
      .number_of_values(1)
      .empty_values(false)
      .help("Generate a JSON report of the verification"))
    .arg(Arg::with_name("custom-header")
      .long("header")
      .takes_value(true)
      .use_delimiter(false)
      .multiple(true)
      .empty_values(false)
      .help("Add a custom header to be included in the calls to the provider. Values must be in the form KEY=VALUE, where KEY and VALUE contain ASCII characters (32-127) only. Can be repeated."))
    .arg(Arg::with_name("no-colour")
      .long("no-colour")
      .visible_alias("no-color")
      .help("Disables ANSI escape codes in the output"))
    .arg(Arg::with_name("ignore-no-pacts-error")
        .long("ignore-no-pacts-error")
        .help("Do not fail if no pacts are found to verify"))
}

#[cfg(test)]
mod test {
  use super::{integer_value, port_value, transport_value};
  use expectest::prelude::*;
  use crate::args::setup_app;

  #[test]
  fn validates_port_value() {
    expect!(port_value("1234")).to(be_ok());
    expect!(port_value("1234x")).to(be_err());
  }

  #[test]
  fn validates_integer_value() {
    expect!(integer_value("3000000")).to(be_ok());
    expect!(integer_value("1234x")).to(be_err());
  }

  #[test]
  fn validates_transport_value() {
    expect!(transport_value("http:1234")).to(be_ok());
    expect!(transport_value("1234x")).to(be_err());
    expect!(transport_value(":1234")).to(be_err());
    expect!(transport_value("x:")).to(be_err());
    expect!(transport_value("x:x")).to(be_err());
    expect!(transport_value("x:1234x")).to(be_err());
  }

  #[test]
  fn verify_cli() {
    setup_app("test", "1.0").debug_assert();
  }
}
