export const getBaseUrl = (): string => {
  return location.origin;
};

export const getWsUrl = (): string => {
  const url = new URL('/messages', location.href);
  url.port = '8080';
  url.protocol = 'ws:';

  return url.href;
};
