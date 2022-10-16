import { readFile } from 'fs/promises';
import * as assert from 'assert/strict';

function unreachable(): never {
  throw new Error('unreachable');
}

class CanonicalDevice {
  private _methods: Record<string, string> = {};

  constructor(public readonly name: string) {}

  registerMethod(method: string, subPath: string = method.toLowerCase()) {
    this._methods[subPath] = method;
  }

  getMethod(subPath: string) {
    let name = this._methods[subPath];
    assert.ok(
      name,
      `Couldn't find canonical name for ${this.name}::${subPath}`
    );
    return name;
  }
}

class CanonicalDevices {
  private _devices: Record<string, CanonicalDevice> = {};

  registerDevice(name: string, path: string = name.toLowerCase()) {
    return (this._devices[path] ??= new CanonicalDevice(name));
  }

  getDevice(path: string) {
    let device = this._devices[path];
    assert.ok(device, `Couldn't find canonical device for ${path}`);
    return device;
  }
}

export async function getCanonicalNames(defaultPath: string) {
  let xml = await readFile('./ASCOM.DriverAccess.xml', 'utf-8');

  let canonical = new CanonicalDevices();

  for (let [
    ,
    deviceName = unreachable(),
    methodName = unreachable()
  ] of xml.matchAll(
    /<member name="[MP]:ASCOM\.DriverAccess\.(\w+?)(?:V\d+)?\.(\w+)[("]/g
  )) {
    let devicePath;
    if (deviceName === 'AscomDriver') {
      deviceName = 'Device';
      devicePath = defaultPath;
    }
    canonical.registerDevice(deviceName, devicePath).registerMethod(methodName);
  }

  return canonical;
}
