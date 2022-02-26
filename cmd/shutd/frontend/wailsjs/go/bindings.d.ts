export interface go {
  "main": {
    "app": {
		Dismiss():Promise<void>
		Greet(arg1:string):Promise<string>
		Snooze():Promise<void>
    },
  }

}

declare global {
	interface Window {
		go: go;
	}
}
