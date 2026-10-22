# Smart features

Some edits need Alakazam to understand your photo first. To smooth skin, color lips or change hair, it has to know where the skin, lips and hair are. Smart models do that. You don't need to do anything but use the edit.

## What they find

- **People:** where each person is, so you can pick one when there are several.
- **Faces:** the eyes, lips, cheeks and other parts that makeup and face edits need.
- **Skin and hair:** so Skin, Teeth Whitening and Hair Color only change what they should.
- **Red eyes:** for **Auto Removal** in Red Eye.

Edits that change the whole photo, like Mood, Adjustments and Special Effects, don't use them.

## How it works

- **They run on your phone.** The smart models are stored on your phone, and your photo is studied there.
- **They run once per photo.** What Alakazam finds in a photo doesn't change, so it finds it once and reuses it for every edit, every preview and every time you reopen the project.
- **They start when the photo opens.** Alakazam gets your photo ready in the background, using the models you already have. If that takes a moment, a **Getting your photo ready** message appears over the photo, and your edits start working by themselves when it's done. A thin bar between the photo and the controls shows how far along it is.
- **They are saved with your project.** What Alakazam finds is stored in the project, so a project opens and replays every edit without running a model again. It opens even on a phone that never downloaded the models.
- **They don't hog your phone.** The models are large, so Alakazam unloads them from memory when the work is done and loads them again only when it needs them.

## Getting the models

The models aren't in the app, because they're large. You download each one once, and it stays on your phone.

- **When you need one:** if an edit needs a model you don't have, Alakazam asks **Download AI models?**, tells you which one and how big it is, and offers **Download**. If you choose **Cancel**, the edit doesn't run. It never opens and silently does nothing.
- **Better body detection:** skin edits work without a model by finding skin by its color. When you open one, Alakazam may suggest **Improve body detection?**, which downloads a model to find bodies more precisely. You can choose **Not now**.

## Managing plugins

Open the **Plugins** tab at the bottom of the home screen and look under **AI models**. Models are grouped by what they find, and each one shows its size and whether it's downloaded.

- **Download** or **Delete** a single model with the button beside it. While one downloads, its button says **Downloading**, and **Waiting** if it's queued.
- **Download all** (down arrow) gets every model you don't have. It shows the total size and asks you to confirm. If one fails, it stops and tells you why.
- **Delete all** (trash can) removes every model, after you confirm. Your projects still open without them, because what the models found is saved.
- **Refresh** (circular arrow) reloads the list of models.

A bar under the title shows download progress, and so does the button of the model that is downloading. They fill as the model arrives, and in **Download all** the bar counts each model as an equal share.

You only need an internet connection to download a model or refresh the list. After that, edits that use the model work offline.
