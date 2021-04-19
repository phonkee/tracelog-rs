#[derive(Clone, Debug, Default)]
pub struct Options {
    pub(crate) header: Header,
    pub(crate) verbose_name: bool,
}

#[derive(Debug, Clone)]
pub struct Header(pub(crate) String);

// set default values for header
impl Default for Header {
    fn default() -> Self {
        Self("x-trace-id".to_owned())
    }
}