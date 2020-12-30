let Cron;

const startDate = new Date("2020-12-01T00:00:00Z");

beforeAll(async () => {
  ({ default: Cron } = await import("@cloudflare/saffron"));
})

it("parses * * * * *", () => {
  let cron = new Cron("* * * * *");
  cron.free();
})

it("parses and describes * * * * *", () => {
  let [cron, description] = Cron.parseAndDescribe("* * * * *");
  try {
    expect(description).toBe("Every minute");
  } finally {
    cron.free();
  }
})

it("throws on invalid cron", () => {
  expect(() => new Cron("invalid")).toThrow();
})

it("gets next time", () => {
  let cron = new Cron("* * * * *");
  try {
    expect(cron.nextFrom(startDate)).toStrictEqual(startDate);
  } finally {
    cron.free();
  }
})

it("gets next after time", () => {
  let cron = new Cron("* * * * *");
  try {
    expect(cron.nextAfter(startDate)).toStrictEqual(new Date("2020-12-01T00:01:00Z"));
  } finally {
    cron.free();
  }
})

it("checks if any values are contained", () => {
  let cron = new Cron("* * 29 2 *");
  try {
    expect(cron.any()).toBe(true)
  } finally {
    cron.free();
  }

  cron = new Cron("* * 31 11 *");
  try {
    expect(cron.any()).toBe(false)
  } finally {
    cron.free();
  }
})

it("iterates after the next 5 minutes", () => {
  let cron = new Cron("* * * * *");
  let arr = [];
  let i = 0;
  let iter = cron.iterAfter(startDate);
  try {
    for (const date of iter) {
      arr.push(date);
      if (++i >= 5) {
        break;
      }
    }
  } finally {
    iter.free();
    cron.free();
  }

  expect(arr).toStrictEqual([
    new Date("2020-12-01T00:01:00Z"),
    new Date("2020-12-01T00:02:00Z"),
    new Date("2020-12-01T00:03:00Z"),
    new Date("2020-12-01T00:04:00Z"),
    new Date("2020-12-01T00:05:00Z"),
  ])
})

it("iterates from the next 5 minutes", () => {
  let cron = new Cron("* * * * *");
  let arr = [];
  let i = 0;
  let iter = cron.iterFrom(startDate);
  try {
    for (const date of iter) {
      arr.push(date);
      if (++i >= 5) {
        break;
      }
    }
  } finally {
    iter.free();
    cron.free();
  }

  expect(arr).toStrictEqual([
    new Date("2020-12-01T00:00:00Z"),
    new Date("2020-12-01T00:01:00Z"),
    new Date("2020-12-01T00:02:00Z"),
    new Date("2020-12-01T00:03:00Z"),
    new Date("2020-12-01T00:04:00Z"),
  ])
})
