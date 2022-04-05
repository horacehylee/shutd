package shutd

import (
	"fmt"
	"time"

	"github.com/go-co-op/gocron"
	"github.com/sirupsen/logrus"
)

const (
	shutdownTag           = "shutdown"
	snoozeNotificationTag = "snoozeNotification"
)

// Config for shutdown scheduler
type Config struct {
	SnoozeInterval int
	StartTime      string
	Notification   struct {
		Before   int
		Duration int
	}
}

// Scheduler for auto shutdown the computer
type Scheduler struct {
	scheduler               *gocron.Scheduler
	logger                  *logrus.Logger
	config                  Config
	shutdownJob             *gocron.Job
	snoozeNotificationJob   *gocron.Job
	shutdownTimeChangedChan chan time.Time
	shutdownTask            SchedulerTask
	snoozeNotificationTask  SchedulerTask
}

// SchedulerTask for scheduler to shutdown or notify for snooze
type SchedulerTask func(s *Scheduler) error

type option func(*Scheduler)

// WithLogger option to allow passing of custom logger
func WithLogger(logger *logrus.Logger) option {
	return func(s *Scheduler) {
		s.logger = logger
	}
}

// WithShutdownTask option to allow passing of custom shutdown task
func WithShutdownTask(t SchedulerTask) option {
	return func(s *Scheduler) {
		s.shutdownTask = t
	}
}

// WithSnoozeNotificationTask option to allow passing of custom snooze notification task, Scheduler.Snooze function can snooze it
func WithSnoozeNotificationTask(t SchedulerTask) option {
	return func(s *Scheduler) {
		s.snoozeNotificationTask = t
	}
}

// NewScheduler to create scheduler to shutdown the computer
func NewScheduler(config Config, options ...option) (*Scheduler, error) {
	s := gocron.NewScheduler(time.Local)
	s.StartAsync()
	s.TagsUnique()

	scheduler := &Scheduler{
		scheduler:               s,
		logger:                  logrus.New(),
		shutdownTimeChangedChan: make(chan time.Time, 1),
		shutdownTask:            newShutdownTask(),
		snoozeNotificationTask:  newNotificationSnoozeTask(),
	}
	for _, o := range options {
		o(scheduler)
	}
	err := scheduler.Configure(config)
	if err != nil {
		return nil, err
	}
	return scheduler, nil
}

// Configure scheduler for updated config
func (s *Scheduler) Configure(config Config) error {
	s.config = config
	s.logger.Infof("config: %+v", config)

	var err error
	err = s.scheduleShutdownJob(s.config.StartTime)
	if err != nil {
		return err
	}
	err = s.scheduleSnoozeNotificationJob()
	if err != nil {
		return err
	}

	s.printJobs()
	return nil
}

// Config get config of the scheduler
func (s *Scheduler) Config() Config {
	return s.config
}

// Logger get logger of the scheduler
func (s *Scheduler) Logger() *logrus.Logger {
	return s.logger
}

// ShutdownTimeChangedChan get channel of latest shutdown time
func (s *Scheduler) ShutdownTimeChangedChan() chan time.Time {
	return s.shutdownTimeChangedChan
}

// ShutdownTime get next shutdown time
func (s *Scheduler) ShutdownTime() (time.Time, error) {
	if s.shutdownJob == nil {
		return time.Time{}, fmt.Errorf("shutdown job is not scheduled")
	}
	return s.shutdownJob.ScheduledTime(), nil
}

// Snooze to delay shutdown time for the computer
func (s *Scheduler) Snooze() error {
	if s.shutdownJob == nil {
		return fmt.Errorf("shutdown job is not scheduled")
	}
	var err error
	delayedTime := s.shutdownJob.ScheduledTime().Add(time.Duration(s.config.SnoozeInterval) * time.Minute)
	err = s.scheduleShutdownJob(delayedTime)
	if err != nil {
		return err
	}
	err = s.scheduleSnoozeNotificationJob()
	if err != nil {
		return err
	}

	s.printJobs()
	return nil
}

func (s *Scheduler) scheduleShutdownJob(shutdownTime interface{}) error {
	if s.shutdownJob == nil {
		j, err := s.scheduler.Every(1).Day().At(shutdownTime).Tag(shutdownTag).Do(func() {
			s.Logger().Info("==========================")
			s.Logger().Info("Shutdown")
			s.Logger().Info("==========================")
			err := s.shutdownTask(s)
			if err != nil {
				s.logger.Errorf("failed to execute shutdown task: %v", err)
			}
		})
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to schedule shutdown job: %v", err)
		}
		s.shutdownJob = j
	} else {
		_, err := s.scheduler.Job(s.shutdownJob).At(shutdownTime).Update()
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to update scheduled shutdown job: %v", err)
		}
	}
	select {
	case s.shutdownTimeChangedChan <- s.shutdownJob.ScheduledTime():
	default:
		// in case no one is waiting for the channel
	}
	return nil
}

func (s *Scheduler) scheduleSnoozeNotificationJob() error {
	if s.shutdownJob == nil {
		return fmt.Errorf("shutdown job is not scheduled")
	}

	notifyTime := s.shutdownJob.ScheduledTime().Add(-time.Duration(s.config.Notification.Before) * time.Minute)
	if s.snoozeNotificationJob == nil {
		s.scheduler.Every(1).Day().StartAt(notifyTime).Tag(snoozeNotificationTag)
	} else {
		s.scheduler.Job(s.snoozeNotificationJob).StartAt(notifyTime)
	}

	if time.Now().After(notifyTime) {
		s.scheduler.StartImmediately()
	}

	if s.snoozeNotificationJob == nil {
		j, err := s.scheduler.Do(func() {
			s.Logger().Info("==========================")
			s.Logger().Info("Snooze notification")
			s.Logger().Info("==========================")
			err := s.snoozeNotificationTask(s)
			if err != nil {
				s.logger.Errorf("failed to execute snooze notification task: %v", err)
			}
		})
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to schedule snooze notification job: %v", err)
		}
		s.snoozeNotificationJob = j
	} else {
		_, err := s.scheduler.Update()
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to update scheduled snooze notification job: %v", err)
		}
	}
	return nil
}

func (s *Scheduler) printJobs() {
	for _, j := range s.scheduler.Jobs() {
		s.logger.Infof("job: %v, scheduled: %v (%v), nextRun: %v (%v), count: %v", j.Tags(), j.ScheduledTime().Format("15:04"), j.ScheduledTime(), j.NextRun().Format("15:04"), j.NextRun(), j.RunCount())
	}
}
