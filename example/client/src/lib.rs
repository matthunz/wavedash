use wavedash::prelude::*;
use wavedash_example_core::ExampleResource;

#[wavedash::main]
fn main(example: ResMut<ExampleResource>) {
    wavedash::dbg(&*example);
}
