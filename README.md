# 🛑 Shutd

Auto shutdown utility tool for Windows with popup and snooze features

## 🛠 Usage

Simply running `shutd` will have the process running and will auto shutdown your computer for you

```
shutd
```

## ⚙ Configuration

Following set of default configurations will be generated under your home directory `%USERPROFILE%/.shutd.yaml`

You may feel free to tweak it for your liking

When you updated the configuration, `shutd` will automatically pick up the latest config, no need to restart it manually

```yaml
startTime: "01:00"
snoozeInterval: 15
notification:
  before: 10
  duration: 10
```

| Property                | Default Value | Remarks                                                             |
| ----------------------- | ------------- | ------------------------------------------------------------------- |
| `startTime`             | "01:00"       | Time for your auto shutdown                                         |
| `snoozeInterval`        | 15            | Minutes that you will snooze for your shutdown                      |
| `notification.before`   | 10            | Minutes before your shutdown for your snooze popup notification     |
| `notification.duration` | 10            | Minutes for your snnoze popup notification to default to not snooze |

## 📃 Logging

Log file will be generated under you home directory `%USERPROFILE%/.shutd.log`

You may troubleshoot your error there if wanted

## 📜 License

Distributed under the MIT License. See `LICENSE.txt` for more information.