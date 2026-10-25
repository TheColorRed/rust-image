# Smart features

Some edits need Alakazam to understand your photo first. To smooth skin, color lips or change hair, it has to know where the skin, lips and hair are. AI models do that. You don't need to do anything but use the edit. Other edits, such as object removal and subject cutouts, also use AI models when you choose them.

## What they find

- **People:** where each person is, so you can pick one when there are several.
- **Faces:** the eyes, lips, cheeks and other parts that makeup and face edits need.
- **Skin and hair:** so beauty edits can target those parts. If the body detail model is missing, skin tools still work using skin color; the model makes the selection more precise.
- **Red eyes:** face landmarks help **Auto Removal** find eyes. You can also tap an eye to fix it manually.
- **Subject edges:** subject cutouts can use an edge refinement model for finer edges, such as hair. Without it, Alakazam uses the available rough selection.
- **Removed objects:** object removal uses a model to fill in the area you paint over.

Most edits that change the whole photo, like Mood and Adjustments, don't use them. Some effects can use subject masks to separate people from the background.

## How it works

- **They run on your phone.** Models are downloaded to your device, and the photo is analyzed there.
- **They run when needed.** Some analyses start in the background when you open a photo, if the needed models are already installed. Other models run when you choose a tool, such as object removal. Analyses that take longer show progress while the app keeps responding.
- **Results are reused and saved.** Alakazam reuses analysis results for edits on the same photo and saves supported masks with the project. Saved projects can replay those edits without running those models again, even on a phone without the models.
- **You can refresh some results.** Where an edit offers a regenerate or rescan action, you can run its analysis again.
- **Models use memory only while needed.** Large models are unloaded when their work is done and loaded again for later work.

## Getting the models

The models aren't bundled in the app because they're large. Download each one you need; it stays on your phone until you delete it.

- **When an edit needs one:** Alakazam asks **Download AI models?**, names the missing models and their combined size, and offers **Download**. If you choose **Cancel**, that requested action won't run.
- **Optional body detection upgrade:** skin edits work without the body detail model, using skin color. When you open a skin edit, Alakazam may offer **Improve body detection?** so it can download the model for more precise results. Choose **Not now** to continue without it.

## Managing plugins

Open the **Plugins** tab at the bottom of the home screen and look under **AI models**. Models are grouped by what they find. Each item shows a description, download size, and whether it's on your device. Some add-ons appear separately under **Add-ons**; see [Plugins](/guide/plugins).

- **Download** or **Delete** a single model with the button beside it. While one downloads, its button says **Downloading**, and **Waiting** if it's queued.
- **Download all** (down arrow) gets every available model you don't have. It shows the total size and asks you to confirm. Downloads happen one at a time; if one fails, the list stops and shows the error.
- **Delete all** (trash can) removes downloaded AI models and confirms how much space will be freed. Projects still replay saved analysis results. If an edit needs a deleted model for new analysis, you can download it again.
- **Refresh** (circular arrow) reloads the list of models.

A progress bar appears while models download, and the active item's button also shows progress. In **Download all**, the bar reflects progress across the queued downloads.

You only need an internet connection to download a model or refresh the list. After that, edits that use the model work offline.
