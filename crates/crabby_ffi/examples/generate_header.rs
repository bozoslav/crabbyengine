//! Prints the generated C header. Redirect stdout to include/crabby.h.
fn main() {
    print!("{}", include_str!(concat!(env!("OUT_DIR"), "/crabby.h")));
}
