pub const PIPELINE_LABEL: &'static str = " Pipeline";
pub const PIPELINE_LAYOUT_LABEL: &'static str = " Pipeline Layout";
pub const VERT_ATTR_LABEL: &'static str = " Vertex Attribute";
pub const INDEX_LABEL: &'static str = " Vertex Index";
pub const INSTANCE_LABEL: &'static str = " Vertex Instance";
pub const BIND_GROUP_LABEL: &'static str = " Bind Group";
pub const BIND_GROUP_LAYOUT_LABEL: &'static str = " Bind Group Layout";
pub const BUFFER_LABEL: &'static str = " Buffer";
pub const TEXTURE_LABEL: &'static str = " Texture";
pub const TEXTURE_VIEW_LABEL: &'static str = " Texture View";
pub const SAMPLER_LABEL: &'static str = " Sampler";

pub const UNNAMED: &'static str = "Unnamed";

pub fn get_default_label<'a>(
    name: &Option<&'a str>,
    postfixs: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut s = name.unwrap_or(UNNAMED).to_owned();
    for postfix in postfixs.into_iter() {
        s.push_str(postfix);
    }
    Some(s)
}
