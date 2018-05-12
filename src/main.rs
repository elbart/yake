extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

use yaml::load_yml_from_file;

mod yaml;
mod yake;

fn main() {
    let yake= load_yml_from_file("Yakefile");

    println!("{:?}", yake.get_targets());
//    println!("{:?}", get_targets(yake.targets));
}
