use udon_core::Parser;

fn main() {
    let inputs = [
        ("|p Hello, !{{user.name}}!\n", "basic interpolation"),
        ("|p !{{first}} and !{{second}}\n", "multiple interpolations"),
        ("|p !{{name | capitalize}}\n", "interpolation with filter"),
        ("!if logged_in\n  |p Welcome\n", "if directive"),
        ("!include partials/header\n", "include directive"),
        ("!:elixir:\n  def hello, do: :world\n", "raw block directive"),
    ];

    for (input, desc) in inputs {
        println!("\n=== {} ===\nInput: {:?}", desc, input);
        Parser::new(input.as_bytes()).parse(|e| {
            println!("  {}", e.format_line());
        });
    }
}
