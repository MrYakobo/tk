use atty::Stream;
use std::path::PathBuf;
use std::process;
use std::process::Command;

use structopt::StructOpt;
use tk;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    // preview the resulting png image in your xdg-open image viewer.
    #[structopt(short, long)]
    preview: bool,

    // "Dimensions" of the resulting image. For example "16x9". This is only used for determining aspect ratio - you have to scale the image yourself afterwards. Defaults to 1x1 (square).
    #[structopt(short, long, default_value = "1x1", required = false)]
    dimensions: String,

    // input file. If not supplied, read from stdin.
    #[structopt(name = "INPUT", default_value = "/dev/stdin", required = false)]
    input: PathBuf,

    /// output file. If outputting PNG-data then it MUST be supplied.
    #[structopt(name = "OUTPUT", default_value = "/dev/stdout", required = false)]
    output: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let tmp_file = PathBuf::from("/tmp/tk.png");
    let stdin = PathBuf::from("/dev/stdin");
    let stdout = PathBuf::from("/dev/stdout");
    let piped = !atty::is(Stream::Stdin);
    let dim = opt.dimensions;

    let arg1 = opt.input.extension();
    let arg2 = opt.output.extension();

    // table lookup to know what to do
    let (input, output, should_encode) = match (piped, arg1, arg2, opt.preview) {
        (true, Some(a), None, false) if a.to_str() == Some("png") => (stdin, opt.input, true), //pipe, png output
        (true, None, None, true) => (stdin, tmp_file, true), //pipe, preview
        (false, Some(a), None, true) if a.to_str() != Some("png") => (opt.input, tmp_file, true), //input, preview
        (false, Some(a), Some(b), _) if a.to_str() != Some("png") && b.to_str() == Some("png") => {
            (opt.input, opt.output, true) //input to png
        }
        (false, Some(a), None, false) if a.to_str() == Some("png") => (opt.input, stdout, false), //decode png
        (false, Some(a), Some(b), false)
            if a.to_str() == Some("png") && b.to_str() != Some("png") =>
        {
            (opt.input, opt.output, false) // decode png to output
        }
        _ => {
            println!("{:?} {:?} {:?} {:?}", piped, arg1, arg2, opt.preview);
            println!(
                r#"
USAGE: tk [INPUT] [OUTPUT] [FLAGS]

FLAGS:

-p: boolean, preview the resulting png image in your xdg-open image viewer.
-d "WxH": string, "Dimensions" of the resulting image. For example "16x9". This is only used for determining aspect ratio - you have to scale the image yourself afterwards. Defaults to 1x1 (square).

EXAMPLE USAGE OF tk:

echo "some input" | tk encoded.png
echo "hello there" | gzip -c | tk -p
tk file.xls -p
tk something.txt encoded.png
tk encoded.png
tk encoded.png output.txt"#
            );
            process::exit(1);
        }
    };

    // println!(
    //     "{:?} {:?} {:?}, should_encode: {:?}",
    //     input, output, dim, should_encode
    // );

    if should_encode {
        if let Err(e) = tk::encode_path(&input, &output, &dim) {
            eprintln!("ERR encoding {:?} to {:?}! Reason: {}", input, output, e);
            process::exit(1);
        }
    } else if let Err(e) = tk::decode_path(&input, &output) {
        eprintln!("ERR decoding {:?} to {:?}! Reason: {}", input, output, e);
        process::exit(1);
    }

    if opt.preview
        && Command::new("imv")
            .arg("-u")
            .arg("nearest_neighbour")
            .arg(&output)
            .spawn()
            .is_err()
    {
        if let Err(f) = Command::new("xdg-open").arg(&output).spawn() {
            eprintln!(
                "Failed previewing output {:?} with imv or xdg-open. Reason: {}",
                output, f
            );
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::PathBuf;
    use tk::parse_aspect;

    fn encode_path_decode() {
        if let Err(e) = tk::encode_path(
            &PathBuf::from("tjenis.txt"),
            &PathBuf::from("out.png"),
            &"1x1".to_string(),
        ) {
            println!("ERR in encode: {}", e);
            std::process::exit(1);
        }

        if let Err(e) = tk::decode_path(&PathBuf::from("out.png"), &PathBuf::from("/dev/stdout")) {
            println!("ERR in decode: {}", e);
            std::process::exit(1);
        }
    }
    fn encode_decode() {
        let input = b"Hello there you little boy";
        let enc = tk::encode(input.to_vec(), parse_aspect(&String::from("1x1")).unwrap()).unwrap();
        let dec = tk::decode(&mut enc.as_slice()).unwrap();
        assert_eq!(input, dec.as_slice());
    }
}
