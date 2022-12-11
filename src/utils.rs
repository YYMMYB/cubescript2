#[macro_export]
macro_rules! builder_set_fn {
    ($fn_name:ident, $field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, $field: $t) -> &mut Self {
            self.$field = Some($field);
            self
        }
    };
}
pub use builder_set_fn;
