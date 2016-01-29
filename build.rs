extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .file("exceptions-wrapper.cpp")
        .flag("-std=c++11")
        .compile("libcpp_exceptions_wrapper.a");
}

