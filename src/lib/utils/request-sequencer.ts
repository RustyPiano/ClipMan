export class RequestSequencer {
  #current = 0;

  next() {
    this.#current += 1;
    return this.#current;
  }

  isCurrent(requestId: number) {
    return requestId === this.#current;
  }
}
