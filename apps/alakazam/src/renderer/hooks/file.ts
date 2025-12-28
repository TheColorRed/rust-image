import { useCallback, useEffect, useMemo, useRef } from 'react';

export const useFiles = () => {
  const eventEmitter = useRef({
    fileOpened: new EventTarget(),
  });

  const openFileDialog = useCallback(async (): Promise<string[] | null> => {
    // return await window.alakazam.openFileDialog();
    return Promise.resolve(null);
  }, []);

  const openImage = useCallback(async (filePath: string): Promise<ImageData> => {
    // return await window.alakazam.openImage(filePath);
    return Promise.resolve(new ImageData(1, 1));
  }, []);

  useEffect(() => {}, []);

  // const openFile = useCallback(async () => {
  //   const filePaths = await window.alakazam.openFileDialog();
  //   if (!filePaths || filePaths.length === 0) return;
  //   for (const filePath of filePaths) {
  //     let p = await window.alakazam.projects.openProject(filePath);
  //     eventEmitter.current.fileOpened.dispatchEvent(
  //       new CustomEvent('fileOpened', {
  //         detail: { filePath, projectId: p.projectId },
  //       }),
  //     );
  //   }
  // }, []);

  // useEffect(() => {
  // const unsubscribe = window.alakazam.onRequestFileOpen(async () => openFile());
  // return () => {
  //   unsubscribe();
  // };
  // }, [openFile]);

  type FileOpenedDetail = { projectId: string; filePath: string };
  const on = useCallback(
    (eventName: keyof typeof eventEmitter.current, handler: (event: CustomEvent<FileOpenedDetail>) => void) => {
      eventEmitter.current[eventName].addEventListener(eventName, handler as EventListener);
      return handler as EventListener;
    },
    [],
  );

  const off = useCallback(
    (eventName: keyof typeof eventEmitter.current, handler: EventListenerOrEventListenerObject) => {
      eventEmitter.current[eventName].removeEventListener(eventName, handler);
    },
    [],
  );

  return useMemo(
    () => ({
      openFileDialog,
      openImage,
      // openFile,
      on,
      off,
    }),
    [openFileDialog, openImage, /*openFile,*/ on, off],
  );
};
