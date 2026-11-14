-- SPDX-FileCopyrightText: The pasejo Authors
-- SPDX-License-Identifier: 0BSD

-- Reads values from a pasejo store through `pasejo secret resolve`, which
-- decrypts the store once per batch however many values fnox asks for.
--
-- A reference is a secret's path, which selects its first line (the
-- password by pasejo's convention), or a path followed by `#FIELD`, which
-- selects the value of that `FIELD: value` line. The path runs up to the
-- last `#`.
--
-- Options (all optional):
--   store    the pasejo store to read; pasejo's default store otherwise
--   bin      the pasejo executable; `pasejo` on PATH otherwise
--   offline  "true" skips the store's pull hooks

local cmd = require("fnox.cmd")
local json = require("fnox.json")

local function command(ctx, ...)
  local argv = { ctx.options.bin or "pasejo" }
  if ctx.options.offline == "true" then
    table.insert(argv, "--offline")
  end
  for _, argument in ipairs({ ... }) do
    table.insert(argv, argument)
  end
  return argv
end

local function reference(ctx, ref)
  local path, field = ref:match("^(.+)#([^#]+)$")
  return {
    secret = path or ref,
    field = field,
    store = ctx.options.store,
  }
end

-- The references double as pasejo's names for the values, so its replies
-- and errors name them as written in fnox.toml. pasejo resolves all
-- references or none: one failing reference fails the batch with pasejo's
-- message instead of being reported as missing.
local function resolve(ctx, refs)
  local request = {}
  for _, ref in ipairs(refs) do
    request[ref] = reference(ctx, ref)
  end
  local result = cmd.run(command(ctx, "secret", "resolve"), { stdin = json.encode(request) })
  if not result.ok then
    error(result.stderr)
  end
  return json.decode(result.stdout)
end

return {
  get = function(ctx, ref)
    return resolve(ctx, { ref })[ref]
  end,
  get_batch = resolve,
  test = function(ctx)
    cmd.exec(command(ctx, "--version"))
  end,
}
