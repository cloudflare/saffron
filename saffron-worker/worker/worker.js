const { describe, next, next_of_many, validate } = wasm_bindgen;

function status(code, text) {
  return new Response(text, { status: code });
}

function corsHeaders() {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": ["POST", "OPTIONS"],
    "Access-Control-Allow-Headers": "Content-Type",
    "Access-Control-Max-Age": 86400
  }
}

function jsonResponseHeaders() {
  return {
    "Content-Type": "application/json",
    ...corsHeaders()
  }
}

function apiResponse(result, success, errors) {
  let json = JSON.stringify({
    result,
    success,
    errors,
    messages: null,
  });
  let status;
  if (success) {
    status = 200;
  } else {
    status = 400;
  }
  return new Response(json, {
    status,
    headers: jsonResponseHeaders(),
  });
}

addEventListener('fetch', event => {
  event.respondWith(handleRequest(event.request).catch((e) => {
    if (env == "dev") {
      console.log(e.stack)
      return status(500, e.message || e.toString());
    } else {
      return status(500, "Internal Server Error");
    }
  }))
})

async function handleRequest(request) {
  await wasm_bindgen(wasm);

  if (request.method == "OPTIONS") {
    return new Response(null, {
      status: 204,
      headers: corsHeaders(),
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
      let success = result == null;
      return apiResponse({}, success, result);
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
      return apiResponse(success ? {
        est_future_times: result.description.est_future_executions,
        description: result.description.text,
      } : {}, success, result.errors || null);
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
      return apiResponse(success ? {
        next: result.next
      } : {}, success, result.errors || null);
    }
    default:
      return status(404, "Not Found");
  }
}
