use std::env;

use libafl_cc::{ClangWrapper, CompilerWrapper, ToolWrapper, LLVMPasses};

pub fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut dir = env::current_exe().unwrap();
        let wrapper_name = dir.file_name().unwrap().to_str().unwrap();

        let is_cpp = match wrapper_name[wrapper_name.len()-2..].to_lowercase().as_str() {
            "cc" => false,
            "++" | "pp" | "xx" => true,
            _ => panic!("Could not figure out if c or c++ wrapper was called. Expected {dir:?} to end with c or cxx"),
        };

        dir.pop();

        let mut cc = ClangWrapper::new();
        if let Some(code) = cc
            .cpp(is_cpp)
            // silence the compiler wrapper output, needed for some configure scripts.
            .silence(true)
            .parse_args(&args)
            .expect("Failed to parse the command line")
            .add_arg("-g") 
            .add_arg("-fsanitize-coverage=edge,no-prune,trace-pc-guard")
            .add_arg("-fsanitize-coverage=trace-cmp")
            .add_arg("-fsanitize-coverage=pc-table")
            .add_pass(LLVMPasses::CmpLogRtn)
            .add_pass(LLVMPasses::CmpLogInstructions)
            .link_staticlib(&dir, "frameshift_afl")
            .run()
            .expect("Failed to run the wrapped compiler")
        {
            std::process::exit(code);
        }
    } else {
        panic!("FBE AFL CC: No Arguments given");
    }
}
