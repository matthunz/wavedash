use wavedash::World;
use wavedash_example_core::Example;

#[wavedash::main]
fn main(world: &mut World) {
    let example = world.resource::<Example>();
    assert_eq!(example.value, 42, "Expected value to be 42!");
}
