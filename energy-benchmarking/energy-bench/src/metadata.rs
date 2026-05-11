pub trait Metadata<const COLS: usize> {
    fn get_header() -> [&'static str; COLS];

    fn get_values(&self) -> [String; COLS];
}

impl Metadata<0> for () {
    fn get_header() -> [&'static str; 0] {
        []
    }

    fn get_values(&self) -> [String; 0] {
        []
    }
}

impl<V: std::fmt::Display> Metadata<1> for V {
    fn get_header() -> [&'static str; 1] {
        [""]
    }

    fn get_values(&self) -> [String; 1] {
        [self.to_string()]
    }
}
