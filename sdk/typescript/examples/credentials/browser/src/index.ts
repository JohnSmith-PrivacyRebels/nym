import { createNymCredentialsClient } from '@nymproject/sdk';
import { appendOutput } from './utils';

async function main() {
  const mnemonic = document.getElementById('mnemonic') as HTMLInputElement;
  const coin = document.getElementById('coin') as HTMLInputElement;
  const client = await createNymCredentialsClient({ isSandbox: true }); // options: {isSandbox?: boolean; networkDetails?: {}}

  const credential = await client.comlink.acquireCredential(coin.value, mnemonic.value, {});
  appendOutput(JSON.stringify(credential, null, 2));
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});
