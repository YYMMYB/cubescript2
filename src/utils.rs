use core::time;
use std::{time::Instant, path::{Path, PathBuf}, env};
use anyhow::*;

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


#[macro_export]
macro_rules! vertex_attribute_layout {
    ($t:ty , struct, { $( $sloc:expr; $field:ident ; $format:ident,)+ }) => {
        vertex_attribute_layout!(@ATTRS offset_of, $t {$($sloc; $field ; $format,)+});
    };
    ($t:ty , tuple, { $( $sloc:expr; $field:ident ; $format:ident,)+ }) => {
        vertex_attribute_layout!(@ATTRS offset_of_tuple, $t {$($sloc; $field ; $format,)+});
    };
    (@ATTRS $offset_fn:tt, $t:ty { $($sloc:expr; $field:ident ; $format:ident,)+ }) => {
        [$(
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::$format,
                offset: memoffset::$offset_fn!($t, $field) as wgpu::BufferAddress,
                shader_location: $sloc,
            }
        ),+]
    };
}

#[macro_export]
macro_rules! vertex_buffer_layout {
    ($t:ty, $step_mode:ident, $attrs:expr) => {{
        VertexBufferLayout {
            array_stride: std::mem::size_of::<$t>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::$step_mode,
            attributes: $attrs,
        }
    }};


}

pub use vertex_attribute_layout;
pub use vertex_buffer_layout;

pub fn get_abs_path(path:impl AsRef<Path>) -> Result<PathBuf>{
    Ok(path.as_ref().canonicalize()?)
    // Ok(env::current_dir()?.join(path))
}