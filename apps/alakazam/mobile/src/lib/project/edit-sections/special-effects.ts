import { EditSection } from '@/src/lib/project/edit-sections';
import { bleachBypassControl } from '@/src/lib/project/edit-sections/special-effects/bleach-bypass';
import { bokehControl } from '@/src/lib/project/edit-sections/special-effects/bokeh';
import { chiaroscuroControl } from '@/src/lib/project/edit-sections/special-effects/chiaroscuro';
import { colorSplashControl } from '@/src/lib/project/edit-sections/special-effects/color-splash';
import { cyanotypeControl } from '@/src/lib/project/edit-sections/special-effects/cyanotype';
import { fadedFilmControl } from '@/src/lib/project/edit-sections/special-effects/faded-film';
import { glitchArtControl } from '@/src/lib/project/edit-sections/special-effects/glitch-art';
import { infraredControl } from '@/src/lib/project/edit-sections/special-effects/infrared';
import { nightVisionControl } from '@/src/lib/project/edit-sections/special-effects/night-vision';
import { popArtControl } from '@/src/lib/project/edit-sections/special-effects/pop-art';
import { splitToneControl } from '@/src/lib/project/edit-sections/special-effects/split-tone';
import { thermalControl } from '@/src/lib/project/edit-sections/special-effects/thermal';
import { vaporwaveControl } from '@/src/lib/project/edit-sections/special-effects/vaporwave';

export const specialEffects: EditSection = {
  key: 'section-special-effects',
  label: 'Special Effects',
  previewThumbnails: true,
  controls: [
    colorSplashControl,
    infraredControl,
    chiaroscuroControl,
    cyanotypeControl,
    fadedFilmControl,
    bleachBypassControl,
    splitToneControl,
    glitchArtControl,
    bokehControl,
    nightVisionControl,
    thermalControl,
    popArtControl,
    vaporwaveControl,
  ],
};
