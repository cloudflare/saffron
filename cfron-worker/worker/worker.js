const { describe, next, next_of_many, validate } = wasm_bindgen;

function status(code, text) {
  return new Response(text, {status: code});
}

addEventListener('fetch', event => {
  try {
    event.respondWith(handleRequest(event.request))
  } catch (e) {
    if (env == "dev") {
      console.log(e.stack)
      return event.respondWith(
        new Response(e.message || e.toString(), {
          status: 500,
        }),
      )
    }
    event.respondWith(status(500, "Internal Server Error"))
  }
})

async function handleRequest(request) {
  await wasm_bindgen(wasm);

  if (request.method == "OPTIONS") {
    return new Response(null, {
      status: 204,
      statusText: "No Content",
      headers: {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": [ "POST", "OPTIONS" ],
        "Access-Control-Allow-Headers": "Content-Type",
        "Access-Control-Max-Age": 86400
      }
    })
  }

  if (request.method != "POST") {
    return status(405, "Method Not Allowed");
  }

  if (request.headers.get("Content-Type") != "application/json") {
    return status(400, "Bad Request");
  }

  const path = new URL(request.url).pathname;
  switch (path) {
    case "/validate": {
      let body;
      try {
        body = await request.json()
      } catch (e) {
        return status(400, "Bad Request");
      }
      let crons = body.crons;
      if (!Array.isArray(crons)) {
        return status(400, "Bad Request");
      }

      let result = validate(crons).errors();
      let response = JSON.stringify({
        result: { },
        success: result == null,
        errors: result || null,
        messages: null,
      });
      return new Response(response, {
        headers: {
          "Content-Type": "application/json",
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": [ "POST", "OPTIONS" ],
          "Access-Control-Allow-Headers": "Content-Type",
          "Access-Control-Max-Age": 86400
        }
      });
    }
    case "/describe": {
      let body;
      try {
        body = await request.json()
      } catch (e) {
        return status(400, "Bad Request");
      }
      let cron = body.cron;
      if (cron == null) {
        return status(400, "Bad Request");
      }
      let result = describe(cron);
      let success = result.errors == null;
      let response = JSON.stringify({
        result: success ? {
          est_future_times: result.description.est_future_executions
        } : { },
        success,
        errors: result.errors || null,
        messages: null,
      });
      let status;
      if (!success) {
        status = 400;
      } else {
        status = 200;
      }
      return new Response(
        response, {
          status,
          headers: {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": [ "POST", "OPTIONS" ],
            "Access-Control-Allow-Headers": "Content-Type",
            "Access-Control-Max-Age": 86400
          }
        }
      );
    }
    case "/next": {
      let body;
      try {
        body = await request.json()
      } catch (e) {
        return status(400, "Bad Request");
      }
      let result;
      if (body.crons != null) {
        result = next_of_many(body.crons);
      } else if (body.cron != null) {
        result = next(body.cron);
      } else {
        return status(400, "Bad Request");
      }
      let success = result.errors == null;
      let response = JSON.stringify({
        result: success ? {
          next: result.next
        } : { },
        success,
        errors: result.errors || null,
        messages: null,
      });
      let statusCode;
      if (!success) {
        statusCode = 400;
      } else {
        statusCode = 200;
      }
      return new Response(
        response, {
          status: statusCode,
          headers: {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": [ "POST", "OPTIONS" ],
            "Access-Control-Allow-Headers": "Content-Type",
            "Access-Control-Max-Age": 86400
          }
        }
      );
    }
    default:
      return status(404, "Not Found");
  }
}
