import { readFileSync, writeFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');
const changelogPath = path.resolve(rootDir, 'CHANGELOG.md');
const outputPath = path.resolve(rootDir, 'release_notes.md');

try {
  const content = readFileSync(changelogPath, 'utf8');
  // Split by line containing exactly '---'
  // Using a robust regex to handle various platform line endings and spacing around '---'
  const parts = content.split(/\r?\n[ \t]*---[ \t]*\r?\n/);
  
  if (parts.length >= 3) {
    const latestChangelog = parts[1].trim();
    writeFileSync(outputPath, latestChangelog, 'utf8');
    console.log('Successfully extracted latest changelog to release_notes.md');
  } else {
    // Try simpler split by '---' if line-based regex didn't yield enough parts
    const simpleParts = content.split('---');
    if (simpleParts.length >= 3) {
      const latestChangelog = simpleParts[1].trim();
      writeFileSync(outputPath, latestChangelog, 'utf8');
      console.log('Successfully extracted latest changelog (using fallback simple split) to release_notes.md');
    } else {
      throw new Error(`CHANGELOG.md does not contain at least two '---' separators. Found ${parts.length} parts.`);
    }
  }
} catch (error) {
  console.error('Error during changelog extraction:', error.message);
  process.exit(1);
}
