import { listLabels, type Label } from '$lib/api';
import { attempt } from './toast.svelte';

/** Collections and tags, with how many books use each. */
class LabelsStore {
  collections = $state<Label[]>([]);
  tags = $state<Label[]>([]);
  loaded = $state(false);
  #seq = 0;

  async refresh() {
    const seq = ++this.#seq;
    const [collections, tags] = await Promise.all([
      attempt('Could not load your collections', () => listLabels('collection')),
      attempt('Could not load your tags', () => listLabels('tag'))
    ]);
    if (seq !== this.#seq) return;
    if (collections) this.collections = collections;
    if (tags) this.tags = tags;
    if (collections && tags) this.loaded = true;
  }
}

export const labels = new LabelsStore();
