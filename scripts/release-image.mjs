import { appendFile, readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

export function releaseImage(repository, release) {
  if (!/^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9._-]*$/i.test(repository || '')) {
    throw new Error('Expected a GitHub owner/repository');
  }
  const tag = release?.tag_name;
  const match = typeof tag === 'string' && tag.match(/^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/);
  if (!match || tag.length > 128 || release.draft === true || typeof release.prerelease !== 'boolean') {
    throw new Error('Publish a release with tag vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-prerelease');
  }
  const version = tag.replace(/^v/, '');
  const tags = [...new Set([tag, version])];
  if (!release.prerelease && !match[4]) tags.push('latest');
  return { image: `ghcr.io/${repository.toLowerCase()}`, tags };
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  if (process.env.GITHUB_EVENT_NAME !== 'release') throw new Error('Only release events may publish an image');
  const event = JSON.parse(await readFile(process.env.GITHUB_EVENT_PATH, 'utf8'));
  if (event.action !== 'published') throw new Error('Only published releases may publish an image');
  const result = releaseImage(process.env.GITHUB_REPOSITORY, event.release);
  await appendFile(process.env.GITHUB_OUTPUT, `image=${result.image}\ntags=${result.tags.join(',')}\n`);
  console.log(`Release image: ${result.image}; tags: ${result.tags.join(', ')}`);
}
