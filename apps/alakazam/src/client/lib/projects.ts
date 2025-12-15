// export interface ImageInfo {
//   /** A unique identifier for the image. */
//   id: string;
//   /** If the image is loaded from a file, this is the file path. */
//   filePath?: string;
//   /** If the image is part of a project, this is the project ID. */
//   projectId?: string;
//   /** The zoom level of the image. Default is 1 (100%). */
//   zoom: number;
//   /** Whether this is the first time the image is being loaded. Used for project images. */
//   firstLoad: boolean;
// }

// const images = [] as ImageInfo[];
// /**
//  * Updates an image at the specified index.
//  * @param id The index of the image.
//  * @param image The new image information.
//  */
// export function updateImage(id: string, image: ImageInfo) {
//   const idx = images.findIndex(img => img.id === id);
//   if (idx === -1) return;
//   images[idx] = image;
// }
// /**
//  * Removes an image at the specified index.
//  * @param id The index of the image to remove.
//  */
// export function removeImage(id: string) {
//   const idx = images.findIndex(img => img.id === id);
//   if (idx === -1) return;
//   images.splice(idx, 1);
// }
// /**
//  * Adds a new image. If the image does not have an ID, a new UUID will be generated.
//  * @param image The image information to add.
//  */
// export function addImage(image: Omit<ImageInfo, 'id'> & { id?: string }) {
//   if (!image.id) image.id = crypto.randomUUID();
//   images.push(image as ImageInfo);
//   return image.id;
// }
// /**
//  * Gets all images that are projects.
//  * @returns An array of all projects.
//  */
// export function getProjects() {
//   return images.filter(img => typeof img.projectId === 'string');
// }
// /**
//  * Retrieves an image at the specified index.
//  * @param id The index of the image.
//  * @returns The image information or null if not found.
//  */
// export function getImage(id?: string | null) {
//   return images.find(img => img.id === id) ?? null;
// }
// /**
//  * Gets the number of projects.
//  * @returns The number of projects.
//  */
// export const numberOfProjects = () => images.filter(img => typeof img.projectId === 'string').length;
// /**
//  * Gets the number of images.
//  * @returns The number of images.
//  */
// export const numberOfImages = () => images.length;
