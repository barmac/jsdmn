import { resolve } from 'node:path';
import { readFileSync } from 'node:fs';

import 'chai/register-expect.js';

const __filename = import.meta.url;

export function getFixture(file) {
  const fixtures = new URL('./fixtures', __filename).pathname;
  const filePath = resolve(fixtures, file);

  return readFileSync(filePath, 'utf-8');
}
