/// <reference lib="deno.ns" />
import { assertEquals, assertRejects } from "@std/assert";
import { SerialQueue } from "./serial_queue.ts";

Deno.test("a late native hide completes before the newer show", async () => {
  const queue = new SerialQueue();
  let enter!: () => void;
  let finish!: () => void;
  const entered = new Promise<void>((resolve) => {
    enter = resolve;
  });
  const release = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const calls: string[] = [];
  let visible = true;
  const hiding = queue.run(async () => {
    calls.push("hide:start");
    enter();
    await release;
    visible = false;
    calls.push("hide:end");
  });
  await entered;
  const showing = queue.run(async () => {
    visible = true;
    calls.push("show");
  });
  assertEquals(calls, ["hide:start"]);
  finish();
  await Promise.all([hiding, showing]);
  assertEquals(calls, ["hide:start", "hide:end", "show"]);
  assertEquals(visible, true);
});

Deno.test("a failed native operation does not poison subsequent requests", async () => {
  const queue = new SerialQueue();
  const failed = queue.run(async () => {
    throw new Error("temporarily unavailable");
  });
  const next = queue.run(async () => "visible");
  await assertRejects(() => failed, Error, "temporarily unavailable");
  assertEquals(await next, "visible");
});

Deno.test("rapid visibility intents can discard stale queued native operations", async () => {
  const queue = new SerialQueue();
  const calls: string[] = [];
  let revision = 0;
  const request = (name: string): Promise<void> => {
    const token = ++revision;
    return queue.run(async () => {
      if (token === revision) calls.push(name);
    });
  };
  await Promise.all([request("show"), request("hide"), request("show")]);
  assertEquals(calls, ["show"]);
});
