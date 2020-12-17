saffron is a cron parser used as part of the backend for Cron Triggers in Cloudflare Workers. It
provides APIs for the complete stack, allowing us to use the same parser everywhere. It's made in
two parts:

1. The parser, which is responsible for reading cron expressions into an easy to understand format,
   which can be simplified with the compiler, or described with `CronExpr::describe`.

2. The compiler, which simplifies expressions into their most compact form. This compact form
   can check if a chrono date time is contained in a given expression in constant time, no matter
   the size of the original expression. It can also be used to get future times that match
   efficiently as an iterator.

The project itself is divided into 4 Rust workspace members:

1. saffron - the parser itself
2. saffron-c - the C API used internally by the Workers API
3. saffron-web - the web API used on the dash in the browser
4. saffron-worker - the Rust Worker which provides the validate/describe endpoint in the dash API on
   the edge as a fallback if WASM can't be used in the browser
