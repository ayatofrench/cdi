use knus;

#[derive(knus::Decode, Debug, PartialEq)]
pub struct Config {
    #[knus(children(name = "service"))]
    pub services: Vec<Service>,
}

#[derive(knus::Decode, Debug, Default, PartialEq, Eq, Clone)]
pub struct Service {
    #[knus(child, unwrap(argument), default)]
    pub cmd: String,
    // #[knus(unwrap(property))]
    // dir: Option<String>,
}

impl Config {
    pub fn parse(filename: &str, text: &str) -> Result<Self, knus::Error> {
        // let _span = tracy_client::span!("Config::parse");
        knus::parse(filename, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn do_parse(text: &str) -> Config {
        Config::parse("test.kdl", text)
            .map_err(miette::Report::new)
            .unwrap()
    }

    #[test]
    fn parse() {
        let parsed = do_parse(
            r##"
            service {
                cmd "pnpm dev"
            }
            "##,
        );

        println!("{:?}", parsed);
    }
}
