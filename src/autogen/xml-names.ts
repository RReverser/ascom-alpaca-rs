import { readFile } from 'fs/promises';
import * as assert from 'assert/strict';
import { parseStringPromise as parseXML } from 'xml2js';

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
  let xmlSrc = await readFile('./ASCOM.DriverAccess.xml', 'utf-8');
  let xml = await parseXML(xmlSrc);

  let canonical = new CanonicalDevices();

  for (let member of xml.doc.members.flatMap((m: any) => m.member)) {
    let nameParts = member.$.name.match(
      /^[MP]:ASCOM\.DriverAccess\.(\w+?)(?:V\d+)?\.(\w+)(?:\(|$)/
    );
    if (!nameParts) continue;
    let [, deviceName, methodName] = nameParts;
    let devicePath;
    if (deviceName === 'AscomDriver') {
      deviceName = 'Device';
      devicePath = defaultPath;
    }
    canonical.registerDevice(deviceName, devicePath).registerMethod(methodName);
  }

  return canonical;
}
