declare const __RELEASE_VERSION__: string;

interface NavigatorUAData {
  getHighEntropyValues(hints: string[]): Promise<{ architecture?: string }>;
}

interface Navigator {
  readonly userAgentData?: NavigatorUAData;
}
