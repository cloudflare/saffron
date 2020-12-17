# 👷‍♀️🦀🕸️ `saffron cron validation Worker`

A fallback Worker for validating, describing, and getting the next execution time of cron strings.

Used in the dash if wasm isn't supported in a user's browser

## 🚴 Usage

---

### Try it locally with `wrangler dev`

---

#### POST to the worker on `/describe`

The worker describes the cron string with a list of estimated future execution times and a human
readable description

```
curl http://localhost:8787/describe -X POST -H "Content-Type: application/json" -d '{"cron": "0 0 * * MON"}'
```

```json
{
  "result": {
    "est_future_times": [
      "2020-10-19T00:00:00.000Z",
      "2020-10-26T00:00:00.000Z",
      "2020-11-02T00:00:00.000Z",
      "2020-11-09T00:00:00.000Z",
      "2020-11-16T00:00:00.000Z"
    ]
  },
  "success": true,
  "errors": null,
  "messages": null
}
```

---

#### POST to the worker on `/validate`

The worker validates multiple cron strings, checking to see if all of them are valid and making sure
no effective duplicates exist.

```
curl http://localhost:8787/validate -X POST -H "Content-Type: application/json" -d '{"crons": ["0 0 * * MON"]}'
```

```json
{
  "result": {},
  "success": true,
  "errors": null,
  "messages": null
}
```

If a duplicate exists, an error is returned detailing which expressions are duplicates

```
curl http://localhost:8787/validate -X POST -H "Content-Type: application/json" -d '{"crons":["0 0 * * MON", "0-0 0-0/1 * * MON,Mon"]}'
```

```json
{
  "result": {},
  "success": false,
  "errors": [
    "Expression '0-0 0-0/1 * * MON,Mon' already exists in the form of '0 0 * * MON'"
  ],
  "messages": null
}
```

---

#### POST to the worker on `/next`

The worker returns the next matching time for the cron expression

```
curl http://localhost:8787/next -X POST -H "Content-Type: application/json" -d '{"cron":"0 0 * * MON"}'
```

```json
{
  "result": {
    "next": "2020-10-19T00:00:00.000Z"
  },
  "success": true,
  "errors": null,
  "messages": null
}
```

---

### 🛠️ Build with `wrangler build`

```
wrangler build
```

---

### 🔬 Deploy with `wrangler deploy`

```
wrangler deploy
```
