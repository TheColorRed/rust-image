# Plugins

Plugins add capabilities to Alakazam after you install them. They are downloaded to your phone from the catalog in the **Plugins** tab, and the app checks each plugin's files before installing it. There are two kinds: AI models that help the app understand a photo, and add-ons that can add editing controls.

Open **Plugins** from the bottom of the home screen. Pull down or tap refresh to update the catalog. Catalog downloads need an internet connection; a plugin package already saved on your device can be installed from a file.

## AI models

AI models help Alakazam find things in a photo, such as people, face parts, skin, hair, or the edge of a subject. An edit that needs a model asks to download it if it isn't already on your phone. You can also manage models in the **AI models** section of the Plugins tab.

Models run on your phone. The app can reuse what a model finds for other edits and save that information with the project, so reopening the project does not require the model to run again. Once downloaded, a model can be used offline. See [Smart features](/guide/smart-features) for details about what the models find and when edits use them.

## Add-ons

Add-ons can contribute editing controls to the editor. After an add-on is installed, its controls appear in the edit panel without restarting the app. An add-on can place controls in an existing section or add its own section. Some add-ons depend on other plugins; Alakazam downloads missing dependencies first.

Add-ons that don't provide an AI model appear in the **Add-ons** section. Use the button beside an add-on to download it, install an available update, or delete it. Deleting an add-on removes the controls it provided. If another installed add-on depends on it, Alakazam explains which add-ons will also be removed before you confirm.

## Install a plugin from a file

Choose **Install from file** at the top of the Plugins screen and select an `.akzx` plugin package. Alakazam checks the package against its `plugin.yml` manifest before installing it. Installing a package with the same plugin ID updates that plugin. Plugins installed from a file that are not in the catalog appear under **Other installed plugins**, where you can check their details or delete them.

## Manage downloads

The screen shows download progress and whether each item is on your phone. Use the delete button beside a model, or the delete-all button in **AI models**, to free space. Projects that already saved their analysis still open and replay without those models.

For development setup and live updates to add-ons on Android, see [Develop a plugin](/guide/plugin-development).
