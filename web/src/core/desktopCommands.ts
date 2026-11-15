import { type InvokeArgs, invoke } from '@tauri-apps/api/core';
import { AppCommands } from '@/bridge/BridgeCommands';

export const commandFailed =
  (command: string) =>
  (error: unknown): void =>
    void AppCommands.recordLog('desktop.ipc', `event=command_failed command=${command} reason="${String(error)}"`).catch(() => false);

export const sendCommand = (command: string, args: InvokeArgs): void => void invoke(command, args).catch(commandFailed(command));
