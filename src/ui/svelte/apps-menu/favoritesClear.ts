interface FavoritesClearOperations<T> {
  getRevision: () => number;
  backup: (snapshot: T) => Promise<void>;
  clear: () => { revision: number; saved: Promise<void> };
  restore: (snapshot: T, error: unknown) => void;
  complete: (snapshot: T) => void;
}

/** Roll back a failed clear without overwriting a later user edit or its status. */
export async function clearFavoritesSafely<T>(snapshot: T, operations: FavoritesClearOperations<T>) {
  const before = operations.getRevision();
  let ownedRevision = before;
  try {
    await operations.backup(snapshot);
    if (operations.getRevision() !== before) return;
    const write = operations.clear();
    ownedRevision = write.revision;
    await write.saved;
    if (operations.getRevision() === ownedRevision) operations.complete(snapshot);
  } catch (error) {
    if (operations.getRevision() === ownedRevision) operations.restore(snapshot, error);
    throw error;
  }
}
