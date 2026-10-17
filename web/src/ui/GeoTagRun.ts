import { Qgc } from '@/bridge/Qgc';
import { mutableStateOf } from '@/kotlin/State';
import { delay, format } from '@/kotlin/Std';
import { type GeoTagTree, documentName, hasTaggedFolder, holdsImages, publishTagged, stageImages, stageLog, taggedOutputDir } from '@/ui/GeoTagStaging';
import { GEOTAG_ROOT, type GeoTagState, geoTagState } from '@/ui/GeoTagView';

const GEOTAG_POLL_MS = 250;
export const DEFAULT_GEOTAG_OUTPUT = 'TAGGED';

export const geoTagOutputText = (output: string | null, images: string | null): string =>
  output ?? (images !== null ? `${images}/${DEFAULT_GEOTAG_OUTPUT}` : `Default: /${DEFAULT_GEOTAG_OUTPUT} subfolder`);

const withFiles = (work: () => Promise<void>): void => {
  if (GeoTagRun.busy.value) return;
  GeoTagRun.busy.value = true;
  void work()
    .catch(() => undefined)
    .then(() => {
      GeoTagRun.busy.value = false;
    });
};

const whileInProgress = async (): Promise<void> => {
  if (GeoTagRun.state.value?.inProgress !== true) return;
  await delay(GEOTAG_POLL_MS);
  await GeoTagRun.refresh();
  return whileInProgress();
};

export const GeoTagRun = {
  state: mutableStateOf<GeoTagState | null>(null),
  note: mutableStateOf<string | null>(null),
  busy: mutableStateOf(false),
  imageTree: mutableStateOf<GeoTagTree | null>(null),
  outputTree: mutableStateOf<GeoTagTree | null>(null),

  async refresh(): Promise<void> {
    GeoTagRun.state.value = geoTagState(await Qgc.get(GEOTAG_ROOT));
  },

  pickDownloadedLog(path: string): void {
    GeoTagRun.note.value = null;
    GeoTagRun.set('logFile', path);
  },

  set(path: string, value: unknown): void {
    void (async () => {
      await Qgc.set(`${GEOTAG_ROOT}.${path}`, value);
      await GeoTagRun.refresh();
    })();
  },

  pickLog(file: File): void {
    withFiles(async () => {
      const staged = await stageLog(file, documentName(file));
      GeoTagRun.note.value = staged === null ? 'That file could not be read.' : null;
      if (staged !== null) await Qgc.set(`${GEOTAG_ROOT}.logFile`, staged);
      await GeoTagRun.refresh();
    });
  },

  pickImages(tree: GeoTagTree): void {
    withFiles(async () => {
      GeoTagRun.imageTree.value = tree;
      const [path, count] = await stageImages(tree);
      GeoTagRun.note.value =
        count === 0 ? 'That folder holds no JPEG, TIFF or DNG images.' : GeoTagRun.outputTree.value === null && (await hasTaggedFolder(tree)) ? GEOTAG_ALREADY_TAGGED : null;
      await Qgc.set(`${GEOTAG_ROOT}.imageDirectory`, path);
      await GeoTagRun.refresh();
    });
  },

  pickOutput(tree: GeoTagTree): void {
    withFiles(async () => {
      GeoTagRun.outputTree.value = tree;
      GeoTagRun.note.value = (await holdsImages(tree)) ? GEOTAG_SAVE_HAS_IMAGES : null;
    });
  },

  cancel(): void {
    void (async () => {
      await Qgc.invoke(`${GEOTAG_ROOT}.cancelTagging`);
      await GeoTagRun.refresh();
    })();
  },

  start(): void {
    withFiles(async () => {
      GeoTagRun.note.value = null;
      const output = await taggedOutputDir();
      await Qgc.set(`${GEOTAG_ROOT}.saveDirectory`, output.absolutePath);
      await Qgc.invoke(`${GEOTAG_ROOT}.startTagging`);
      await GeoTagRun.refresh();
      await whileInProgress();
      const finished = GeoTagRun.state.value;
      const target = GeoTagRun.outputTree.value ?? GeoTagRun.imageTree.value;
      if (finished !== null && !finished.previewMode && finished.tagged > 0 && target !== null) {
        const published = await publishTagged(output, target, GeoTagRun.outputTree.value === null ? DEFAULT_GEOTAG_OUTPUT : null);
        GeoTagRun.note.value = publishedNote(published, finished.tagged);
      }
    });
  },
};

export const publishedNote = (published: number, tagged: number): string | null =>
  published === tagged ? null : `Only ${published} of the ${tagged} tagged images could be written to the chosen folder.`;

const GEOTAG_OFFSET_LIMIT = 3600.0;
const DECIMAL = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/;

export const parsedOffset = (typed: string): number | null => {
  const text = typed.trim().replaceAll(',', '.');
  if (text === '') return 0.0;
  const value = DECIMAL.test(text) ? Number(text) : NaN;
  const decimals = text.includes('.') ? text.slice(text.indexOf('.') + 1) : '';
  return !Number.isNaN(value) && value >= -GEOTAG_OFFSET_LIMIT && value <= GEOTAG_OFFSET_LIMIT && decimals.length <= 1 ? value : null;
};

export const GEOTAG_ALREADY_TAGGED = 'Images have already been tagged. Existing images will be removed.';
export const GEOTAG_SAVE_HAS_IMAGES = 'The save folder already contains images.';

export const shownOffset = (seconds: number): string => format('%.1f', seconds);
