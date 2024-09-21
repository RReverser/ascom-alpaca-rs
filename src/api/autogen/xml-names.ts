import { readFile } from 'fs/promises';
import * as assert from 'assert/strict';
import { parseStringPromise as parseXML } from 'xml2js';

export class CanonicalDevice {
  private _methods: Record<string, string> = {};

  constructor(public readonly name: string) {}

  registerMethod(method: string) {
    this._methods[method.toLowerCase()] = method;
  }

  getMethod(subPath: string) {
    let name = this._methods[subPath];
    assert.ok(
      name,
      `Couldn't find canonical name for ${this.name}::${subPath}`
    );
    return name;
  }

  getMethods() {
    return Object.values(this._methods);
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

  getDevices() {
    return Object.values(this._devices);
  }
}

export const canonicalDevices = new CanonicalDevices();

{
  let xmlSrc = await readFile('./ASCOM.DeviceInterfaces.xml', 'utf-8');
  let xml = await parseXML(xmlSrc);

  for (let member of xml.doc.members.flatMap((m: any) => m.member)) {
    let nameParts = member.$.name.match(
      /^[MP]:ASCOM\.DeviceInterface\.I(\w+)V\d+\.(\w+)/
    );
    if (!nameParts) continue;
    let [, deviceName, methodName] = nameParts;
    canonicalDevices.registerDevice(deviceName).registerMethod(methodName);
  }

  // Find common methods and combine into a new Device object.
  let commonMethods = Object.values(canonicalDevices.getDevices())
    .map(device => new Set(device.getMethods()))
    .reduce((a, b) => a.intersection(b));

  let commonDevice = canonicalDevices.registerDevice('Device', '{device_type}');
  for (let method of commonMethods) {
    commonDevice.registerMethod(method);
  }
}
