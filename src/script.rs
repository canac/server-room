use std::fmt;

pub struct Script {
    pub name: String,
    pub command: String,
}

impl fmt::Display for Script {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}: {}", self.name, self.command)
    }
}
