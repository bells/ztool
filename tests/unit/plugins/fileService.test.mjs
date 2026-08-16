import assert from "node:assert/strict";
import test from "node:test";
import {
  createFileConversionService,
  FILE_CONVERSION_COMMANDS,
  FILE_CONVERSION_JOB_UPDATED_EVENT,
  normalizeFileConversionError,
} from "/private/tmp/zero-tests/plugins/file/fileServiceCore.js";

test("file service uses stable commands and exact camelCase input envelopes", async () => {
  const calls = [];
  let eventRegistration;
  const service = createFileConversionService(
    async (command, args) => {
      calls.push([command, args]);
      return [];
    },
    async (eventName, handler) => {
      eventRegistration = [eventName, handler];
      return () => {};
    },
  );

  await service.getCapabilities();
  await service.chooseInputs();
  await service.inspectInputs(["/tmp/报告.pdf"]);
  await service.enqueue([{ sourcePath: "/tmp/报告.pdf" }]);
  await service.listJobs();
  await service.start();
  await service.cancel("job-1");
  await service.remove("job-1");
  await service.retry("job-1");
  await service.clearCompleted();
  await service.open("job-1");
  await service.reveal("job-1");
  await service.subscribe(() => {});

  assert.deepEqual(calls, [
    [FILE_CONVERSION_COMMANDS.capabilities, undefined],
    [FILE_CONVERSION_COMMANDS.choose, undefined],
    [FILE_CONVERSION_COMMANDS.inspect, { input: { sourcePaths: ["/tmp/报告.pdf"] } }],
    [FILE_CONVERSION_COMMANDS.enqueue, { input: { items: [{ sourcePath: "/tmp/报告.pdf" }] } }],
    [FILE_CONVERSION_COMMANDS.list, undefined],
    [FILE_CONVERSION_COMMANDS.start, undefined],
    [FILE_CONVERSION_COMMANDS.cancel, { input: { jobId: "job-1" } }],
    [FILE_CONVERSION_COMMANDS.remove, { input: { jobId: "job-1" } }],
    [FILE_CONVERSION_COMMANDS.retry, { input: { jobId: "job-1" } }],
    [FILE_CONVERSION_COMMANDS.clearCompleted, undefined],
    [FILE_CONVERSION_COMMANDS.open, { input: { jobId: "job-1" } }],
    [FILE_CONVERSION_COMMANDS.reveal, { input: { jobId: "job-1" } }],
  ]);
  assert.equal(eventRegistration[0], FILE_CONVERSION_JOB_UPDATED_EVENT);
});

test("file errors narrow structured values and safely normalize unknown failures", () => {
  assert.deepEqual(
    normalizeFileConversionError({
      code: "engineUnavailable",
      message: "Install LibreOffice.",
      retryable: true,
      providerId: "libreOffice",
      ignored: "/private/path",
    }),
    {
      code: "engineUnavailable",
      message: "Install LibreOffice.",
      retryable: true,
      providerId: "libreOffice",
    },
  );
  assert.equal(normalizeFileConversionError(new Error("bridge failed")).code, "internal");
  assert.equal(normalizeFileConversionError({ code: "madeUp" }).code, "internal");
});
