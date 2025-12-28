import { contextBridge } from 'electron';

export interface AlakazamHistoryApi {}

contextBridge.exposeInMainWorld('alakazamHistory', {} as AlakazamHistoryApi);
