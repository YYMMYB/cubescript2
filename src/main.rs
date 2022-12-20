use cubescript2::{render::*, window::run};
fn main() {
    env_logger::init();

    let error_handle = async {
        match run().await {
            Ok(_) => {
                println!("Success!");
            }
            Err(err) => {
                println!("{:?}", err);
            }
        }
    };

    pollster::block_on(error_handle);
}