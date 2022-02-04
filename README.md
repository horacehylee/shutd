# 🛑 Shutd

Auto shutdown utility tool for Windows with popup and snooze features

## 🛠 Usage

Simply running `shutd` will have the process running and will auto shutdown your computer for you

```
shutd
```

## ⚙ Configuration

Following set of default configurations will be generated under home directory `%USERPROFILE%/.shutd.yaml`

Feel free to tweak it for your liking

After updated the configuration, `shutd` will automatically pick up the latest config, no need to restart it manually

```yaml
startTime: "01:00"
snoozeInterval: 15
notification:
  before: 10
  duration: 10
```

| Property                | Default Value | Remarks                                                             |
| ----------------------- | ------------- | ------------------------------------------------------------------- |
| `startTime`             | "01:00"       | Time for auto shutdown                                         |
| `snoozeInterval`        | 15            | Minutes that will snooze for shutdown                      |
| `notification.before`   | 10            | Minutes before shutdown for snooze popup notification     |
| `notification.duration` | 10            | Minutes for snnoze popup notification to default to not snooze |

## 📃 Logging

Log file will be generated under you home directory `%USERPROFILE%/.shutd.log`

Troubleshoot error there if wanted

## 📜 License

Distributed under the MIT License. See [LICENSE](./LICENSE) for more information.