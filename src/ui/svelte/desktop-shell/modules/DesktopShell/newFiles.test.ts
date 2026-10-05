import assert from "node:assert/strict";
import test from "node:test";
import { newFileBaseName, renameSelectionEnd } from "./newFiles.ts";

test("new file names sanitize native labels without losing Unicode", () => {
  assert.equal(newFileBaseName('新建 Word/Excel: 文档*. ', '新建文件'), '新建 Word Excel 文档');
  assert.equal(newFileBaseName('\0 . ', '新建文件'), '新建文件');
  assert.equal(Array.from(newFileBaseName('😀'.repeat(100), 'File')).length, 80);
});

test("renaming selects the stem, but never a folder's partial name", () => {
  assert.equal(renameSelectionEnd('新建文本文档.txt', false), 6);
  assert.equal(renameSelectionEnd('notes.v2.md', false), 8);
  assert.equal(renameSelectionEnd('folder.v2', true), 9);
  assert.equal(renameSelectionEnd('.gitignore', false), 10);
  assert.equal(renameSelectionEnd('README', false), 6);
});
