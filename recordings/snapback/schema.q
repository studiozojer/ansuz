-- Verified identities arrive through the ATproto authorization service.
-- No password or guest account creation.

-- Original evidence. Interpretation belongs in separate annotations.
table recordings:
  owner: principal
  digest: text 64..64
  label: text 1..100
  expected: text <=1000
  element: int 0..4
  captured: time
  frames: int 1..2400
  model: text <=100
  trace: file <=2097152 ('application/json')
  at: time default now
  unique owner, digest
  immutable owner, digest, label, expected, element, captured, frames, model, trace, at
  read <- .owner = viewer or exists readers[.owner, viewer]
  insert <- .owner = viewer
  online only

table readers:
  owner: principal
  reader: principal
  unique owner, reader
  immutable owner, reader
  read <- .owner = viewer or .reader = viewer
  insert <- .owner = viewer
  delete <- .owner = viewer
  online only

invariant recordings.read only .owner = viewer or exists readers[.owner, viewer]
invariant recordings.insert only .owner = viewer
invariant readers.read only .owner = viewer or .reader = viewer
invariant readers.insert only .owner = viewer
invariant readers.delete only .owner = viewer

query history(owner: principal, c: cursor ?):
  return recordings where .owner = owner last 30 by .at after c
query findTake(owner: principal, digest: text 64..64):
  return recordings[owner, digest]
query access():
  return readers where .owner = viewer first 100 by .id
mutation archive(digest: text 64..64, label: text 1..100, expected: text <=1000, element: int 0..4, captured: time, frames: int 1..2400, model: text <=100, trace: file <=2097152 ('application/json')):
  return insert recordings { owner: viewer, digest, label, expected, element, captured, frames, model, trace }
mutation allowReader(reader: principal):
  return insert readers { owner: viewer, reader } if absent
mutation removeReader(reader: principal):
  delete readers[viewer, reader]
query sharedLibraries(c: cursor ?):
  return readers where .reader = viewer first 50 by .id after c

query currentIdentity():
  return { did: viewer }
