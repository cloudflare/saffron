# cfron-web

`cfron-web` is a set of web bindings of `cfron` compiled to wasm for use with webpack.

## 🚴 Usage

### Parse a cron string and check if it contains a specific time

```ts
import Cron from "@cloudflare/cfron";

let cron = new Cron("0 0 L 2 *");

console.log(cron.contains(new Date("2020-02-28T00:00:00"))); // false
console.log(cron.contains(new Date("2020-02-29T00:00:00"))); // true
console.log(cron.contains(new Date("2021-02-28T00:00:00"))); // true

// be sure to free the wasm memory when you're done with the expression!
cron.free();
```

### Parse a cron string and get the next 5 matching times

```ts
import Cron from "@cloudflare/cfron";

let cron = new Cron("0 0 L * *");
let iter = cron.iterFrom(new Date("1970-01-01T00:00:00"));

let array = [];
let i = 0;
for (let next of iter) {
  array[i] = next;
  if (++i >= 5) {
    break;
  }
}
console.log(array);

// be sure to free the wasm memory when you're done with the iterator!
iter.free();
cron.free();
```
