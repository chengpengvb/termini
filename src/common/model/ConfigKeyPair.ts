import { isEmpty } from "../util/StringUtil";

export interface ConfigKeyPair {
  id?: string;
  name?: string;
  public_key?: string;
  private_key?: string;
  passphrase?: string;
  certificate?: string;
}

export function createEmptyConfigKeyPair(): ConfigKeyPair {
  return {};
}

export function verifyConfigKeyPair(value: ConfigKeyPair): boolean {
  if (isEmpty(value.name)) {
    return false;
  }
  if (value.name.length > 64) {
    return false;
  }
  return !isEmpty(value.private_key);
}